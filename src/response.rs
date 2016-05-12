use common::Result;
use std::io;
use std::io::{BufReader, Read, Write, BufRead};
use std::fs::File;
use std::path::Path;
use std::net::TcpStream;
use std::collections::HashMap;


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

  pub fn download(&mut self, destination_path: &Path) -> Result<()> {
    let response_head = try!(self.get_head());
    match response_head.content_length() {
      Some(content_length) => {
        let mut destination = try_str!(File::create(destination_path));
        self.download_fixed_bytes(content_length, &mut destination)
      },
      None =>
        if response_head.is_chunked() {
          let mut destination = try_str!(File::create(destination_path));
          self.download_chunked(&mut destination)
        } else {
          Err("Unsupported response. Supported response must be either chunked or have Content-Length".to_owned())
        }
    }
  }

  fn download_chunked(&mut self, destination: &mut Write) -> Result<()> {
    loop {
      let chunk_size = try!(self.read_line_r_n()
        .and_then(|line| str_err!(u64::from_str_radix(&line, 16))));

      try!(self.download_fixed_bytes(chunk_size, destination));
      try!(self.eat_r_n());

      if chunk_size == 0 {
        return Ok(());
      }
    }
  }

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
    let headers: io::Result<Vec<String>> = self.reader.by_ref().lines()
      .take_while(|res| match *res {
        Ok(ref line) if !line.is_empty() => true,
        _ => false
      }).collect();

    str_err!(headers)
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
