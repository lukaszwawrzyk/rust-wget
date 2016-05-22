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
use path_resolve::get_destination_path;

enum Status {
  AlreadyDownloaded,
  Success(PathBuf),
  Redirect(Url),
}

pub struct Http {
  options: Options,
}

// todo fix ++ bug
// todo slow down progress refresh
// todo test retries

impl Http {
  pub fn new(options: Options) -> Http {
    Http { options: options }
  }

  pub fn download_all(&self) -> CompoundResult<()> {
    for url in &self.options.urls {
      println!("\n{}", try!(self.download_one(url)));
    }
    Ok(())
  }

  fn download_one(&self, url: &Url) -> CompoundResult<String> {
    let mut progress = Progress::new();
    self.download_one_recursive(url, &mut progress, 0)
  }

  fn download_one_recursive(&self, url: &Url, progress: &mut Progress, initial_tries: u64) -> CompoundResult<String> {
    let destination_path = try!(get_destination_path(url, &self.options));

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
          CompoundError::ConnectionError(_) | CompoundError::TemporaryServerError =>
            if tries_limited { tries += 1 },
          fatal_error =>
            fail!(fatal_error),
        },
      }
    }

    fail!(format!("Failed after {} tries", try_limit));
  }

  fn try_download_one(&self, destination_path: &Path, progress: &mut Progress, url: &Url) -> CompoundResult<Status> {
    return if Self::file_exists(destination_path) {
      let file_size = try!(Self::file_size(destination_path));
      let response = try!(Request::send_with_range_from(url, &self.options, file_size));
      self.maybe_show_response(&response);

      return match response.status {
        StatusCode::RangeNotSatisfiable => Ok(Status::AlreadyDownloaded),
        StatusCode::PartialContent => {
          if self.options.continue_download {
            progress.try_set_predownloaded(file_size);
          }
          Self::download_body(response, || OpenOptions::new().append(true).open(destination_path), progress, destination_path)
        },
        StatusCode::Ok => if !self.options.continue_download {
          Self::download_body(response, || File::create(destination_path), progress, destination_path)
        } else {
          fail!(CompoundError::ServerDoesNotSupportContinuation)
        },
        other => handle_errors_and_redirects(other, &response),
      }
    } else {
      let response = try!(Request::send_default(url, &self.options));
      self.maybe_show_response(&response);

      match response.status {
        StatusCode::Ok => Self::download_body(response, || File::create(destination_path), progress, destination_path),
        other => handle_errors_and_redirects(other, &response),
      }
    };

    fn handle_errors_and_redirects(response_status: StatusCode, response: &Response) -> CompoundResult<Status> {
      match response_status.class() {
        StatusClass::Redirection  => {
          let redirect_url = try!(extract_redirect_url(response));
          Ok(Status::Redirect(redirect_url))
        },
        StatusClass::ClientError => fail!(CompoundError::BadResponse(format!("Status {}", response_status).to_string())),
        StatusClass::ServerError => fail!(CompoundError::TemporaryServerError),
        _ => fail!(CompoundError::BadResponse(format!("Unknown status code {}", response_status).to_string())),
      }
    }

    fn extract_redirect_url(response: &Response) -> CompoundResult<Url> {
      response.headers.get::<header::Location>()
        .and_then(|loc| Url::parse(&loc.0).ok())
        .ok_or(CompoundError::BadResponse("Redirect response contains no Location header".to_string()))
    }
  }

  fn maybe_show_response(&self, response: &Response) -> () {
    if self.options.show_response {
      println!("{} {}", response.version, response.status);
      println!("{}", response.headers);
    }
  }

  fn file_exists(path: &Path) -> bool {
    fs::metadata(path).is_ok()
  }

  fn file_size(path: &Path) -> CompoundResult<u64> {
    let size = try!(fs::metadata(path).map(|md| md.len()));
    Ok(size)
  }

  fn download_body<F, W: Write>(mut response: Response, write_supplier: F, progress: &mut Progress, destination_path: &Path) -> CompoundResult<Status>
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
        ResponseBuffer::read_fixed_bytes(&mut response, content_length, &mut destination, progress)
      },
      None => {
        if is_chunked {
          let mut destination = try!(write_supplier());
          progress.try_initialize_indeterminate();
          ResponseBuffer::read_chunked(&mut response, &mut destination, progress)
        } else {
          fail!(CompoundError::UnsupportedResponse);
        }
      },
    }.map(|_| Status::Success(destination_path.to_path_buf()))
  }
}
