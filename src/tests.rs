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


// const BUFFER_SIZE: usize = 512;
//
// impl Response {
//   pub fn new(socket: TcpStream) -> Response {
//     Response { socket: socket }
//   }
//
//   pub fn download(&mut self, destination: &mut Write) -> Result<()> {
//     let response_head = try!(self.get_head());
//
//     let content_length = try!(response_head.content_length().ok_or("No content length".to_owned()));
//
//     let mut buffer: [u8; BUFFER_SIZE] = [0; BUFFER_SIZE];
//
//     let mut total_bytes_read: u64 = 0;
//     loop {
//       let bytes_read: usize = try!(str_err!(self.socket.read(&mut buffer)));
//
//       if bytes_read == 0 {
//         if total_bytes_read != content_length {
//           return Err(format!("Failed to read expected number of bytes. Read {} of {}", total_bytes_read, content_length));
//         } else {
//           return Ok(());
//         }
//       }
//
//       try!(str_err!(destination.write_all(&buffer[0..bytes_read])));
//
//       total_bytes_read += bytes_read as u64;
//
//       if total_bytes_read >= content_length {
//         return Ok(());
//       }
//     }
//   }
//
//   fn get_head(&self) -> Result<ResponseHead> {
//     self.read_raw_head().and_then(|raw_head| {
//       match &raw_head[..] {
//         [ref status_line, raw_headers..] =>
//           Self::get_status_code(&status_line).map(|code| {
//             let headers = Self::get_header_map(raw_headers);
//             ResponseHead {
//               status_code: code,
//               headers: headers
//             }
//           }),
//         _ => Err("Invalid response format".to_owned()),
//       }
//     })
//   }
//
//   fn get_status_code(line: &String) -> Result<u16> {
//     line.split_whitespace().nth(1)
//       .ok_or("Bad response - no status code found".to_owned())
//       .and_then(|code| code.parse::<u16>()
//         .map_err(|_| "Bad response - invalid status code".to_owned()))
//   }
//
//   fn get_header_map(header_lines: &[String]) -> HashMap<String, String> {
//     header_lines.into_iter().flat_map(|line| {
//       let splitted: Vec<&str> = line.split(": ").collect();
//       match &splitted[..] {
//         [key, value] => Some((key.to_string(), value.to_string())),
//         _ => None
//       }
//     }).collect()
//   }
//
//   fn read_raw_head(&self) -> Result<Vec<String>> {
//     let response = BufReader::new(&self.socket);
//
//     let headers: io::Result<Vec<String>> = response.lines()
//       .take_while(|res| match *res {
//         Ok(ref line) if !line.is_empty() => true,
//         _ => false
//       }).collect();
//
//     str_err!(headers)
//   }
// }
//

pub type Result<T> = result::Result<T, String>;

macro_rules! str_err {
  ($e:expr) => {
    $e.map_err(|err| err.to_string());
  };
}


fn download() -> Result<()> {
  let mut destination = try!(str_err!(File::create(Path::new("google.html"))));
  let mut socket = try!(str_err!(TcpStream::connect("google.com:80")));
  try!(str_err!(socket.write("GET / HTTP/1.1\r\nHost: www.google.com\r\nAccept: */*\r\n\r\n".as_bytes())));

  let mut reader = BufReader::new(&socket);
  {
    let mut headers: Vec<String> = Vec::new();
    loop {
      let mut line = String::new();
      try!(str_err!(reader.read_line(&mut line)));

      if line.ends_with("\r\n") {
        let len = line.len();
        line.truncate(len - 2);
      }

      println!("READ: |{}|", line);

      if line.is_empty() {
        break;
      } else {
        headers.push(line);
      }
    }

    let hdr = headers;

    println!("{:?}", hdr);

    let mut buf1: [u8; 10] = [0; 10];
    try!(str_err!(reader.read_exact(&mut buf1)));
    println!("{:?}", String::from_utf8_lossy(&buf1));
  }

  Ok(())
}



fn main() {
  match download() {
    Ok(_) => println!("OK"),
    Err(e) => println!("{}", e),
  }
}
