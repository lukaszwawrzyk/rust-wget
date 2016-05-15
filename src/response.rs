use common::Result;
use common;
use std::io;
use std::io::{BufReader, Read, Write, BufRead};
use std::net::TcpStream;
use std::collections::HashMap;
use progress::Progress;

pub struct ResponseHead {
  pub status_code: u16,
  headers: HashMap<String, String>,
  raw: Vec<String>,
}

impl ResponseHead {
  pub fn content_length(&self) -> Option<u64> {
    self.headers.get("Content-Length").and_then(|num| num.parse::<u64>().ok())
  }

  pub fn is_chunked(&self) -> bool {
    self.headers.get("Transfer-Encoding").map_or(false, |encoding| encoding == "chunked")
  }

  pub fn print_raw(&self) -> () {
    for line in &self.raw {
      println!("{}", line);
    }
  }
}

const BUFFER_SIZE: usize = 16 * 1024;

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

  pub fn read_chunked(&mut self, destination: &mut Write, progress: &mut Progress) -> Result<()> {
    loop {
      let chunk_size = try!(self.read_line_r_n().and_then(|line| str_err!(u64::from_str_radix(&line, 16))));

      progress.chunk_start(chunk_size);

      try!(self.read_fixed_bytes(chunk_size, destination, progress));
      try!(self.eat_r_n());

      if chunk_size == 0 {
        return Ok(());
      }
    }
  }

  pub fn read_fixed_bytes(&mut self, expected_length: u64, destination: &mut Write, progress: &mut Progress) -> Result<()> {
    let mut total_bytes_read: u64 = 0;
    loop {
      let bytes_left = expected_length - total_bytes_read;

      let bytes_read: usize = try_str!(self.reader.by_ref().take(bytes_left).read(&mut self.buffer[..]));

      if bytes_read == 0 {
        if total_bytes_read != expected_length {
          return Err(format!("Failed to read expected number of bytes. Read {} of {}", total_bytes_read, expected_length));
        } else {
          return Ok(());
        }
      }

      try_str!(destination.write_all(&self.buffer[0..bytes_read]));

      progress.update(bytes_read as u64);

      total_bytes_read += bytes_read as u64;

      if total_bytes_read >= expected_length {
        return Ok(());
      }
    }
  }

  pub fn read_head(&mut self) -> Result<ResponseHead> {
    self.read_raw_head().and_then(|raw_head| {
      match &raw_head[..] {
        [ref status_line, raw_headers..] =>
          Self::get_status_code(&status_line).map(|code| {
            let headers = common::parse_header_lines(raw_headers);
            ResponseHead {
              status_code: code,
              headers: headers,
              raw: raw_head.clone(),
            }
          }),
        _ => Err("Invalid response format".to_owned()),
      }
    })
  }

  fn get_status_code(line: &String) -> Result<u16> {
    line.split_whitespace().nth(1)
      .ok_or(format!("Bad response: no status code found in {}", line).to_owned())
      .and_then(|code| code.parse::<u16>()
        .map_err(|_| format!("Bad response - invalid status code {}", code).to_owned()))
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
