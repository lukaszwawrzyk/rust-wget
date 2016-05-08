#![feature(advanced_slice_patterns, slice_patterns)]

extern crate url;

use std::io;
use std::io::{BufReader, Read, Write, BufRead};
use std::fs::File;
use std::path::Path;
use std::net::TcpStream;
use url::Url;
use std::result;
use std::collections::HashMap;

// HELPERS

pub type Result<T> = result::Result<T, String>;

macro_rules! str_err {
  ($e:expr) => {
    $e.map_err(|err| err.to_string());
  };
}

// REQUEST

struct Request {
  content: String,
}

impl Request {
  fn format(url: &Url) -> Result<Request> {
    url.host_str()
      .map(|host| format!("GET {} HTTP/1.1\r\nHost: {}\r\nAccept: */*\r\n\r\n", url.path(), host))
      .map(|request| Request { content: request })
      .ok_or("No host found in url".to_owned())
  }

  fn send(&self, socket: &mut TcpStream) -> Result<()> {
    let bytes = self.content.as_bytes();
    str_err!(socket.write(bytes).map(|_| ()))
  }
}

// RESPONSE

struct ResponseHead {
  status_code: u16,
  headers: HashMap<String, String>,
}

impl ResponseHead {
  fn content_length(&self) -> Option<u64> {
    self.headers.get("Content-Length").and_then(|num| num.parse::<u64>().ok())
  }
}

pub struct Response {
  socket: TcpStream,
}

const BUFFER_SIZE: usize = 512;

impl Response {
  pub fn new(socket: TcpStream) -> Response {
    Response { socket: socket }
  }

  pub fn download(&mut self, destination: &mut Write) -> Result<()> {
    let response_head = try!(self.get_head());

    let content_length = try!(response_head.content_length().ok_or("No content length".to_owned()));

    let mut buffer: [u8; BUFFER_SIZE] = [0; BUFFER_SIZE];

    let mut total_bytes_read: u64 = 0;
    loop {
      let bytes_read: usize = try!(str_err!(self.socket.read(&mut buffer)));

      if bytes_read == 0 {
        if total_bytes_read != content_length {
          return Err(format!("Failed to read expected number of bytes. Read {} of {}", total_bytes_read, content_length));
        } else {
          return Ok(());
        }
      }

      try!(str_err!(destination.write_all(&buffer[0..bytes_read])));

      total_bytes_read += bytes_read as u64;

      if total_bytes_read >= content_length {
        return Ok(());
      }
    }
  }

  fn get_head(&self) -> Result<ResponseHead> {
    self.read_raw_head().and_then(|raw_head| {
      match &raw_head[..] {
        [ref status_line, raw_headers..] =>
          Self::get_status_code(&status_line).map(|code| {
            let headers = Self::get_header_map(raw_headers);
            ResponseHead {
              status_code: code,
              headers: headers
            }
          }),
        _ => Err("Invalid response format".to_owned()),
      }
    })
  }

  fn get_status_code(line: &String) -> Result<u16> {
    line.split_whitespace().nth(1)
      .ok_or("Bad response - no status code found".to_owned())
      .and_then(|code| code.parse::<u16>()
        .map_err(|_| "Bad response - invalid status code".to_owned()))
  }

  fn get_header_map(header_lines: &[String]) -> HashMap<String, String> {
    header_lines.into_iter().flat_map(|line| {
      let splitted: Vec<&str> = line.split(": ").collect();
      match &splitted[..] {
        [key, value] => Some((key.to_string(), value.to_string())),
        _ => None
      }
    }).collect()
  }

  fn read_raw_head(&self) -> Result<Vec<String>> {
    let response = BufReader::new(&self.socket);

    let headers: io::Result<Vec<String>> = response.lines()
      .take_while(|res| match *res {
        Ok(ref line) if !line.is_empty() => true,
        _ => false
      }).collect();

    str_err!(headers)
  }
}




fn download(source_url: &str, target_file: &str) -> Result<()> {
  let mut destination = try!(str_err!(File::create(Path::new(target_file))));

  let url = try!(parse_url(source_url));
  let mut socket = try!(connect(&url));

  let request = try!(Request::format(&url));
  try!(request.send(&mut socket));

  let mut response = Response::new(socket);

  return response.download(&mut destination);
}

fn connect(url: &Url) -> Result<TcpStream> {
  fn default_port(url: &Url) -> result::Result<u16, ()> {
    match url.scheme() {
      "https" => Ok(443),
      "http" => Ok(80),
      _ => Err(()),
    }
  }

  let socket = url.with_default_port(default_port)
    .and_then(|address| TcpStream::connect(address)); // TODO see if can simplify

  str_err!(socket)
}

fn parse_url(url: &str) -> Result<Url> {
  str_err!(Url::parse(url))
}


fn main() {
  let source_url = "http://google.com/";
  let target_file = "google.html";

  match download(source_url, target_file) {
    Ok(headers) => println!("{:?}", headers),
    Err(e) => println!("{}", e),
  }
}
