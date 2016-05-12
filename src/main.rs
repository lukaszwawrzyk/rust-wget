#![feature(advanced_slice_patterns, slice_patterns)]

extern crate url;

#[macro_use]
mod common;
mod request;
mod response;

use std::env;
use response::Response;
use request::Request;
use common::Result;
use std::path::Path;
use std::net::TcpStream;
use url::Url;
use std::result;



const DEFAULT_FILE_NAME: &'static str = "out";

fn download(source_url: &str, file_name_opt: Option<String>) -> Result<String> {
  let url = try_str!(Url::parse(source_url));

  let mut socket = try!(connect(&url));

  let file_name = file_name_opt.unwrap_or(file_name_from_url(&url));
  let destination_path = Path::new(&file_name);


  let request = try!(Request::format(&url));
  try!(request.send(&mut socket));

  let mut response = Response::new(socket);

  return response.download(&destination_path).map(|_| format!("Downloaded to {}", destination_path.to_string_lossy()).to_string());


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
}

fn main() {
  let args: Vec<String> = env::args().collect();

  let result = match &args[..] {
    [_, ref source_url] =>
      download(source_url, None),
    [_, ref source_url, ref destination_file] =>
      download(source_url, Some(destination_file.to_string())),
    [ref name, ..] =>
      Err(format!("Usage: {} <url> [dest_file]", name).to_owned()),
    _ =>
      Err("Invalid argments".to_owned()),
  };

  match result {
    Ok(msg) => println!("{}", msg),
    Err(e) => println!("{}", e),
  }
}


// TODO show progress in %, kb of all, speed
// TODO check https
// TODO check status code to see if should look for eof or abort
