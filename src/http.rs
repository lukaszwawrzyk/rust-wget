use options::Options;
use common::Result;
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

  pub fn download_all(&self) -> Result<String> {
    self.download_one(&self.options.urls[0])
  }

  fn download_one(&self, url: &Url) -> Result<String> {
      let mut progress = Progress::new();
      let mut socket = try!(Self::connect(url));

      let basic_file_name = Self::file_name_from_url(url);

// TODO continue ++++++ on progress bar
// TODO if range request is sent to server with chunked encoding it will send 200 + Content-Length 0 but no chunked header but message still will be chunked - handle this

      let file_metadata = fs::metadata(Path::new(&basic_file_name));
      if self.options.continue_download && file_metadata.is_ok() {
        let file_size = file_metadata.unwrap().len();

        try!(Request::send_with_range_from(&mut socket, url, &self.options, file_size));

        let mut response = Response::new(socket);

        let response_head = try!(response.read_head(self.options.show_response));

        if response_head.status_code == 416 {
          Ok("File is already fully downloaded. Nothing to do.".to_owned())
        } else if response_head.status_code == 206 {
          let destination_path = Path::new(&basic_file_name);
          let result = Self::dowload_body(response_head, response, || OpenOptions::new().append(true).open(destination_path), &mut progress);
          result.map(|_| format!("Downloaded to {}", destination_path.to_string_lossy()).to_string())
        } else if response_head.status_code == 200 {
          let file_name = try!(self.backup_file_name(&basic_file_name));
          let destination_path = Path::new(&file_name);
          let result = Self::dowload_body(response_head, response, || File::create(destination_path), &mut progress);
          result.map(|_| format!("Downloaded to {}", destination_path.to_string_lossy()).to_string())
        } else {
          Err(format!("Invalid status code: {}", response_head.status_code).to_string())
        }
      } else {
        let file_name = try!(self.backup_file_name(&basic_file_name));

        try!(Request::send_default(&mut socket, url, &self.options));

        let mut response = Response::new(socket);

        let response_head = try!(response.read_head(self.options.show_response));

        let destination_path = Path::new(&file_name);
        let result = Self::dowload_body(response_head, response, || File::create(destination_path), &mut progress);
        result.map(|_| format!("Downloaded to {}", destination_path.to_string_lossy()).to_string())
      }
  }

  fn connect(url: &Url) -> Result<TcpStream> {
    fn default_port(url: &Url) -> result::Result<u16, ()> {
      match url.scheme() {
        "http" => Ok(80),
        _ => Err(()),
      }
    }

    let socket = url.with_default_port(default_port).and_then(TcpStream::connect);

    str_err!(socket)
  }

  fn file_name_from_url(url: &Url) -> String {
    url.path_segments()
      .and_then(|segments| segments.last())
      .map(|s| s.to_string())
      .and_then(|s| if s.is_empty() { None } else { Some(s) })
      .unwrap_or(DEFAULT_FILE_NAME.to_string())
  }

  fn dowload_body<F, W: Write>(response_head: ResponseHead, mut response: Response, write_supplier: F, progress: &mut Progress) -> Result<()>
  where F: Fn() -> io::Result<W> {
    match response_head.content_length() {
      Some(content_length) => {
        let mut destination = try_str!(write_supplier());
        progress.chunk_start(content_length);
        response.read_fixed_bytes(content_length, &mut destination, progress)
      },
      None => {
        if response_head.is_chunked() {
          let mut destination = try_str!(write_supplier());
          response.read_chunked(&mut destination, progress)
        } else {
          Err("Unsupported response. Supported response must be either chunked or have Content-Length".to_owned())
        }
      },
    }
  }

  fn backup_file_name(&self, basic_name: &str) -> Result<String> {
    let dir = try_str!(fs::read_dir(Path::new("./")));
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
            try_str!(Self::shift_names(basic_name, limit));
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
