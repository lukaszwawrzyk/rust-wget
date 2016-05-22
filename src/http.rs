use options::Options;
use common::{CompoundResult, CompoundError};
use std::path::{Path, PathBuf};
use response::ResponseBuffer;
use request::Request;
use hyper::Url;
use std::fs;
use std::io;
use std::io::Write;
use progress::Progress;
use std::fs::{File, OpenOptions};
use hyper::header;
use hyper::status::{StatusCode, StatusClass};
use hyper::client::response::Response;

enum Status {
  AlreadyDownloaded,
  Success(PathBuf),
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
    self.download_one_recursive(url, &mut progress, 0)
  }

  fn download_one_recursive(&self, url: &Url, progress: &mut Progress, initial_tries: u64) -> CompoundResult<String> {
    let destination_path = try!(self.get_destination_path(url));

    let tries_limited = self.options.tries.is_some();
    let try_limit = self.options.tries.unwrap_or(0);
    let mut tries = initial_tries;

    while !tries_limited || tries < try_limit {
      match self.try_download_one(&destination_path, progress, url) {
        Ok(status) => match status {
          Status::AlreadyDownloaded => return Ok("File already downloaded, nothing to do.".to_string()),
          Status::Success(ref path) => return Ok(format!("Downloaded to {}", path.to_string_lossy()).to_string()),
          Status::Redirect(ref new_url) => return self.download_one_recursive(new_url, progress, tries),
        },
        Err(error) => match error {
          CompoundError::ConnectionError(_) |
          CompoundError::TemporaryServerError =>
            if tries_limited { tries += 1 },
            fatal_error => fail!(fatal_error),
        },
      }
    }

    fail!(format!("Failed after {} tries", try_limit));
  }

// TODO readd print response

  fn try_download_one(&self, destination_path: &Path, progress: &mut Progress, url: &Url) -> CompoundResult<Status> {
    return if Self::file_exists(destination_path) {
      let file_size = try!(Self::file_size(destination_path));
      let response = try!(Request::send_with_range_from(url, &self.options, file_size));

      return match response.status {
        StatusCode::RangeNotSatisfiable => Ok(Status::AlreadyDownloaded),
        StatusCode::PartialContent => {
          if self.options.continue_download {
            progress.try_set_predownloaded(file_size);
          }
          Self::download_body(response, || OpenOptions::new().append(true).open(destination_path), progress)
            .map(|_| Status::Success(destination_path.to_path_buf()))
        },
        StatusCode::Ok => if !self.options.continue_download {
          Self::download_body(response, || File::create(destination_path), progress)
            .map(|_| Status::Success(destination_path.to_path_buf()))
        } else {
          fail!(CompoundError::ServerDoesNotSupportContinuation)
        },
        other => handle_errors_and_redirects(other, &response),
      }
    } else {
      let response = try!(Request::send_default(url, &self.options));

      match response.status {
        StatusCode::Ok => Self::download_body(response, || File::create(destination_path), progress)
          .map(|_| Status::Success(destination_path.to_path_buf())),
        other => handle_errors_and_redirects(other, &response),
      }
    };

    fn handle_errors_and_redirects(response_status: StatusCode, response: &Response) -> CompoundResult<Status> {
      match response_status.class() {
        StatusClass::Redirection  => {
          let redirect_url = try!(response.headers.get::<header::Location>()
            .and_then(|loc| Url::parse(&loc.0).ok())
            .ok_or(CompoundError::BadResponse("Redirect response contains no Location header".to_string())));
          Ok(Status::Redirect(redirect_url))
        },
        StatusClass::ClientError => fail!(CompoundError::BadResponse(format!("Status {}", response_status).to_string())),
        StatusClass::ServerError => fail!(CompoundError::TemporaryServerError),
        _ => fail!(CompoundError::BadResponse(format!("Unknown status code {}", response_status).to_string())),
      }
    }
  }

  fn file_exists(path: &Path) -> bool {
    fs::metadata(path).is_ok()
  }

  fn file_size(path: &Path) -> CompoundResult<u64> {
    let size = try!(fs::metadata(path).map(|md| md.len()));
    Ok(size)
  }

  fn get_destination_path(&self, url: &Url) -> CompoundResult<PathBuf> {
    let basic_file_name = Self::file_name_from_url(url);
    if self.should_continue_download(&basic_file_name) {
      Ok(PathBuf::from(&basic_file_name))
    } else {
      Ok(PathBuf::from(try!(self.backup_file_name(&basic_file_name))))
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

  fn download_body<F, W: Write>(mut response: Response, write_supplier: F, progress: &mut Progress) -> CompoundResult<()>
  where F: Fn() -> io::Result<W> {
    let content_length_opt = response.headers.get::<header::ContentLength>().map(|len| len.0);
    let is_chunked = match response.headers.get::<header::TransferEncoding>() {
      Some(&header::TransferEncoding(ref encodings)) if encodings.contains(&header::Encoding::Chunked) => true,
      _ => false,
    };

    match content_length_opt {
      Some(content_length) => {
        let mut destination = try!(write_supplier());
        progress.try_initialize_sized(content_length);
        let mut source = ResponseBuffer::new();
        source.read_fixed_bytes(&mut response, content_length, &mut destination, progress)
      },
      None => {
        if is_chunked {
          let mut destination = try!(write_supplier());
          progress.try_initialize_indeterminate();
          let mut source = ResponseBuffer::new();
          source.read_chunked(&mut response, &mut destination, progress)
        } else {
          fail!(CompoundError::UnsupportedResponse);
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
