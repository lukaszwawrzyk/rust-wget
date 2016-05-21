use options::Options;
use common::{CompoundResult, CompoundError};
use std::path::{Path, PathBuf};
use response::{Response, ResponseHead};
use request::Request;
use std::net::TcpStream;
use url::Url;
use std::fs;
use std::result;
use std::io;
use std::io::Write;
use progress::Progress;
use std::fs::{File, OpenOptions};
use std::time::Duration;

enum Status {
  AlreadyDownloaded,
  Success(Path),
  Redirect(Url),
}

pub struct Http {
    options: Options,
}

const DEFAULT_FILE_NAME: &'static str = "out";

impl Http {
  pub fn new(options: Options) -> Http {
      Http {
          options: options,
      }
  }

  pub fn download_all(&self) -> CompoundResult<String> {
    self.download_one(&self.options.urls[0])
  }

  fn download_one(&self, url: &Url) -> CompoundResult<String> {
      let mut progress = Progress::new();
      self.download_one_recursive(url, progress, 0)
  }

  fn download_one_recursive(&self, url: &Url, progress: &mut Progress, initial_tries: u64) -> CompoundResult<String> {
      let mut destination_path = self.get_destination_path(url);

      let tries_limited = self.options.tries.is_some();
      let try_limit = self.options.tries.unwrap_or(0);
      let mut tries = initial_tries;

      while !tries_limited || tries < try_limit {
          match self.try_download_one(&destination_path, &progress, url) {
              Ok(status) => match status {
                  Status::AlreadyDownloaded => return Ok("File already downloaded, nothing to do."),
                  Status::Success(ref path) => return Ok(format!("Downloaded to {}", path.to_string_lossy()).to_string()),
                  Status::Redirect(ref new_url) => return self.download_one(new_url, progress, tries),
              },
              Err(error) => match error {
                  CompoundError::ConnectionError(_) |
                  CompoundError::TemporaryServerError =>
                    if tries_limited { tries++ },
                  fatal_error => fail!(fatal_error),
              },
          }
      }

      fail!(format!("Failed after {} tries", try_limit));
  }

  fn try_download_one(&self, destination_path: &Path, progress: &Progress, url: &Url) -> CompoundResult<Status> {
    let mut socket = try!(self.connect(url));

    if Self::file_exists(destination_path) {
      let file_size = try!(file_size(destination_path));
      try!(Request::send_with_range_from(&mut socket, url, &self.options, file_size));

      let mut response = Response::new(socket);
      let response_head = try!(response.read_head(self.options.show_response));

      return match response_head.status_code {
        416 => Ok(Status::AlreadyDownloaded)),
        206 => {
            if self.options.continue_download {
                progress.try_set_predownloaded(file_size);
            }
            Self::dowload_body(response_head, response, || OpenOptions::new().append(true).open(destination_path), &mut progress)
        },
        200 => if !self.options.continue_download {
          Self::dowload_body(response_head, response, || File::create(destination_path), &mut progress);
        } else {
          fail!(CompoundError::ServerDoesNotSupportContinuation);
        },
        other => handle_errors_and_redirects(other, &response_head),
      }
    } else {
        try!(Request::send_default(&mut socket, url, &self.options));

        let mut response = Response::new(socket);
        let response_head = try!(response.read_head(self.options.show_response));

        match response_head.status_code {
          200 => Self::dowload_body(response_head, response, || File::create(destination_path), &mut progress),
          other => handle_errors_and_redirects(other, &response_head),
        }
    };

    fn handle_errors_and_redirects(response_status: u16, response_head: &ResponseHead) -> CompoundResult<Status> {
      match response_status {
        301 | 302 | 303 | 307 | 308  => {
          let redirect_url = try!(response_head.location().unwrap_or(CompoundError::BadResponse("Redirect response contains no Location header")));
          Ok(Status::Redirect(redirect_url))
        },
        status @ 400 ... 499 => fail!(CompoundError::BadResponse(format!("Status code {}", status).to_string())),
        500 ... 511 => fail!(CompoundError::TemporaryServerError),
        other => fail!(CompoundError::BadResponse(format!("Unknown status code {}", other).to_string())),
      }
    }
  }

  fn file_exists(path: &Path) -> bool {
    fs::metadata(path).is_ok()
  }

  fn file_size(path: &Path) -> CompoundResult<u64> {
    try!(fs::metadata(path).map(Metadata::len))
  }

  fn get_destination_path(&self, url: &Url) -> Path {
    let basic_file_name = Self::file_name_from_url(url);
    let destination_path = if self.should_continue_download(&basic_file_name) {
      Path::new(&basic_file_name)
    } else {
      Path::new(try!(self.backup_file_name(&basic_file_name)))
    }
  }

  fn file_name_from_url(url: &Url) -> String {
    url.path_segments()
      .and_then(|segments| segments.last())
      .map(|s| s.to_string())
      .and_then(|s| if s.is_empty() { None } else { Some(s) })
      .unwrap_or(DEFAULT_FILE_NAME.to_string())
  }

  fn should_continue_download(&self, file_name: &str) -> bool {
    let file_metadata = fs::metadata(Path::new(file_name));

    self.options.continue_download && file_metadata.is_ok()
  }

  fn connect(&self, url: &Url) -> CompoundResult<TcpStream> {
    fn default_port(url: &Url) -> result::Result<u16, ()> {
      match url.scheme() {
        "http" => Ok(80),
        _ => Err(()),
      }
    }

    let socket = try!(url.with_default_port(default_port).and_then(TcpStream::connect));
    let timeout = self.options.timeout_secs.map(Duration::from_secs);
    try!(socket.set_read_timeout(timeout));
    try!(socket.set_write_timeout(timeout));

    Ok(socket)
  }

  fn dowload_body<F, W: Write>(response_head: ResponseHead, mut response: Response, write_supplier: F, progress: &mut Progress) -> CompoundResult<()>
  where F: Fn() -> io::Result<W> {
    match response_head.content_length() {
      Some(content_length) => {
        let mut destination = try!(write_supplier());
        progress.try_initialize_sized(content_length);
        response.read_fixed_bytes(content_length, &mut destination, progress)
      },
      None => {
        if response_head.is_chunked() {
          let mut destination = try!(write_supplier());
          progress.try_initialize_indeterminate();
          response.read_chunked(&mut destination, progress)
        } else {
          fail!(CompoundError::UnsupportedResponse)
        }
      },
    }
  }

  fn backup_file_name(&self, basic_name: &str) -> CompoundResult<String> {
    let dir = try!(fs::read_dir(Path::new("./")));
    let files: Vec<String> = dir
      .flat_map(|r| r.ok())
      .flat_map(|entry| entry.file_name().to_str().map(|s| s.to_string()))
      .collect::<Vec<String>>();
    if !files.contains(&basic_name.to_string()) {
      return Ok(basic_name.to_string());
    }

    let prefix: &str = &format!("{}.", basic_name);
    let mut current_indices: Vec<u64> = files.iter()
      .filter(|s| s.starts_with(prefix))
      .map(|s| (&s[(basic_name.len() + 1)..]).to_string())
      .flat_map(|s| s.parse::<u64>().ok())
      .collect();
    current_indices.sort();

    match self.options.backup_limit {
      None => {
        let next_index = (1..).zip(current_indices.iter())
          .find(|&(expected_index, &actual_index)| actual_index > expected_index)
          .map(|(free_index, _)| free_index)
          .unwrap_or(current_indices.len() as u64 + 1);

        Ok(format!("{}.{}", basic_name, next_index).to_string())
      },
      Some(limit) => {
        let missing_index = (1..(limit + 1)).zip(current_indices.iter())
          .find(|&(expected_index, &actual_index)| actual_index > expected_index)
          .map(|(free_index, _)| free_index);

        match missing_index {
          Some(next_index) => Ok(format!("{}.{}", basic_name, next_index).to_string()),
          None => {
            try!(Self::shift_names(basic_name, limit));
            Ok(basic_name.to_string())
          },
        }
      },
    }
  }

  fn shift_names(basic_name: &str, limit: u64) -> io::Result<()> {
    try!(fs::remove_file(to_path(basic_name, limit)));
    for i in (1..limit).rev() {
      try!(fs::rename(to_path(basic_name, i), to_path(basic_name, i + 1)));
    }
    try!(fs::rename(Path::new(basic_name), to_path(basic_name, 1)));
    return Ok(());

    fn to_path(basic_name: &str, num: u64) -> PathBuf {
      let name = format!("{}.{}", basic_name, num);
      PathBuf::from(name)
    }
  }
}
