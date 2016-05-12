extern crate url;

use common::Result;
use std::net::TcpStream;
use url::Url;
use std::io::Write;

pub struct Request {
  content: String,
}

impl Request {
  pub fn format(url: &Url) -> Result<Request> {
    url.host_str()
      .map(|host| format!("GET {} HTTP/1.1\r\nHost: {}\r\nAccept: */*\r\n\r\n", url.path(), host))
      .map(|request| Request { content: request })
      .ok_or("No host found in url".to_owned())
  }

  pub fn send(&self, socket: &mut TcpStream) -> Result<()> {
    let bytes = self.content.as_bytes();
    str_err!(socket.write(bytes).map(|_| ()))
  }
}
