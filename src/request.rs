extern crate url;

use common::Result;
use std::net::TcpStream;
use url::Url;
use std::io::Write;
use options::Options;
use options::Credentials;
use rustc_serialize::base64::*;
use std::collections::HashMap;
use common;

pub struct Request {
  content: String,
}

impl Request {
  pub fn default(url: &Url, options: &Options) -> Result<Request> {
    Self::build(url, options, HashMap::new())
  }

  pub fn with_range_from(url: &Url, options: &Options, range_from: u64) -> Result<Request> {
    let mut headers = HashMap::new();
    headers.insert("Range".to_string(), format!("bytes={}-", range_from).to_string());
    Self::build(url, options, headers)
  }

  fn build(url: &Url, options: &Options, special_headers: HashMap<String, String>) -> Result<Request> {
    let head_line = format!("GET {} HTTP/1.1", url.path()).to_string();

    let mut headers: HashMap<String, String> = HashMap::new();

    // basic headers
    let host = try!(url.host_str().ok_or("No host found in url".to_owned()));
    headers.insert("Host".to_string(), host.to_string());
    headers.insert("Accept".to_string(), "*/*".to_string());

    // credentials
    if let Some(ref credentials) = options.credentials {
      let auth_header = Self::format_auth_header(&credentials);
      headers.insert("Authorization".to_string(), auth_header);
    }

    // additional headers (internal)
    for (k, v) in special_headers {
      headers.insert(k, v);
    }

    // headers from user that may override current
    let extra_headers = common::parse_header_lines(&options.headers[..]);
    for (k, v) in extra_headers {
      headers.insert(k, v);
    }

    Ok(Self::format_request(head_line, headers))
  }

  fn format_request(head_line: String, headers: HashMap<String, String>) -> Request {
    let mut lines: Vec<String> = headers.iter().map(|(k, v)| format!("{}: {}", k, v).to_string()).collect();
    lines.insert(0, head_line);
    lines.push("\r\n".to_string());

    Request {
      content: lines.join("\r\n"),
    }
  }

  fn format_auth_header(credentials: &Credentials) -> String {
    let cred_str = format!("{}:{}", credentials.user, credentials.password);
    let cred_bytes = cred_str.as_bytes();
    let as_base64 = cred_bytes.to_base64(Config {
        char_set: CharacterSet::Standard,
        newline: Newline::CRLF,
        pad: false,
        line_length: None
    });

    format!("Basic {}", as_base64).to_string()
  }

  pub fn send(&self, socket: &mut TcpStream) -> Result<()> {
    let bytes = self.content.as_bytes();
    str_err!(socket.write(bytes).map(|_| ()))
  }
}
