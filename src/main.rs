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

macro_rules! try_str {
  ($e:expr) => {
    try!($e.map_err(|err| err.to_string()));
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

  fn is_chunked(&self) -> bool {
    self.headers.get("Transfer-Encoding").map_or(false, |encoding| encoding == "chunked")
  }
}

const BUFFER_SIZE: usize = 512;

pub struct Response {
  reader: BufReader<TcpStream>,
  buffer: [u8; BUFFER_SIZE],
}

impl Response {
  pub fn new(socket: TcpStream) -> Response {
    Response {
      reader: BufReader::new(socket),
      buffer: [0; BUFFER_SIZE],
    }
  }

  pub fn download(&mut self, destination: &mut Write) -> Result<()> {
    let response_head = try!(self.get_head());
    match response_head.content_length() {
      Some(content_length) =>
        self.download_fixed_bytes(content_length, destination),
      None =>
        if response_head.is_chunked() {
          self.download_chunked(destination)
        } else {
          Err("Unsupported response. Supported response must be either chunked or have Content-Length".to_owned())
        }
    }
  }

  fn download_chunked(&mut self, destination: &mut Write) -> Result<()> {
    loop {
      let chunk_size = try!(self.read_line_r_n()
        .and_then(|line| str_err!(u64::from_str_radix(&line, 16))));
      println!("Chunk size: {:?}", chunk_size);

      println!("Downloading {:?} bytes", chunk_size);
      try!(self.download_fixed_bytes(chunk_size, destination));
      println!("Eating rn");
      try!(self.eat_r_n());

      if chunk_size == 0 {
        println!("CHUNK SIZE WAS 0");
        return Ok(());
      }
    }
  }

  // TODO ZROBIC B REF DO HEADERÓW

  fn download_fixed_bytes(&mut self, expected_length: u64, destination: &mut Write) -> Result<()> {
    let mut total_bytes_read: u64 = 0;
    loop {
      let bytes_left = expected_length - total_bytes_read;

      let bytes_read: usize = if bytes_left < BUFFER_SIZE as u64 {
        try_str!(self.reader.by_ref().take(bytes_left).read(&mut self.buffer[..]))
      } else {
        try_str!(self.reader.read(&mut self.buffer[..]))
      };

      if bytes_read == 0 {
        if total_bytes_read != expected_length {
          return Err(format!("Failed to read expected number of bytes. Read {} of {}", total_bytes_read, expected_length));
        } else {
          return Ok(());
        }
      }

      try_str!(destination.write_all(&self.buffer[0..bytes_read]));

      total_bytes_read += bytes_read as u64;

      if total_bytes_read >= expected_length {
        return Ok(());
      }
    }
  }

  fn get_head(&mut self) -> Result<ResponseHead> {
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

  fn read_raw_head(&mut self) -> Result<Vec<String>> {
    let mut headers: Vec<String> = Vec::new();
    loop {
      let line = try!(self.read_line_r_n());

      if line.is_empty() {
        return Ok(headers);
      } else {
        headers.push(line);
      }
    }
  }

  fn read_line_r_n(&mut self) -> Result<String> {
    let mut line = String::new();
    try_str!(self.reader.read_line(&mut line));

    if line.ends_with("\r\n") {
      let len = line.len();
      line.truncate(len - 2);
    }

    Ok(line)
  }

  fn eat_r_n(&mut self) -> Result<()> {
    let line = try!(self.read_line_r_n());
    if line.is_empty() {
      Ok(())
    } else {
      Err(format!("Expected empty line, found {}", line).to_owned())
    }
  }
}

fn download(source_url: &str, target_file: &str) -> Result<()> {
  let url = try_str!(Url::parse(source_url));
  let mut socket = try!(connect(&url));

  let mut destination = try_str!(File::create(Path::new(target_file)));

  let request = try!(Request::format(&url));
  try!(request.send(&mut socket));

  let mut response = Response::new(socket);

  return response.download(&mut destination);

  fn connect(url: &Url) -> Result<TcpStream> {
    fn default_port(url: &Url) -> result::Result<u16, ()> {
      match url.scheme() {
        "https" => Ok(443),
        "http" => Ok(80),
        _ => Err(()),
      }
    }

    let socket = url.with_default_port(default_port).and_then(TcpStream::connect);

    str_err!(socket)
  }
}

fn main() {
  let source_url = "http://jigsaw.w3.org/HTTP/ChunkedScript";
  let target_file = "img.jpg";

  match download(source_url, target_file) {
    Ok(_) => println!("Download success!"),
    Err(e) => println!("{}", e),
  }
}
