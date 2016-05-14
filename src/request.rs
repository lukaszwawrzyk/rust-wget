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
  pub fn format(url: &Url, options: &Options) -> Result<Request> {
    let head_line = format!("GET {} HTTP/1.1", url.path()).to_string();

    let mut headers: HashMap<String, String> = HashMap::new();
    let host = try!(url.host_str().ok_or("No host found in url".to_owned()));
    headers.insert("Host".to_string(), host.to_string());
    headers.insert("Accept".to_string(), "*/*".to_string());

    if let Some(ref credentials) = options.credentials {
      let auth_header = Self::format_auth_header(&credentials);
      headers.insert("Authorization".to_string(), auth_header);
    }

    let extra_headers = common::parse_header_lines(&options.headers[..]);
    for (k, v) in extra_headers {
      headers.insert(k, v);
    }

    let mut lines: Vec<String> = headers.iter().map(|(k, v)| {
        format!("{}: {}", k, v).to_string()
    }).collect();
    lines.insert(0, head_line);
    lines.push("\r\n".to_string());

    Ok(Request{
      content: lines.join("\r\n"),
    })
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
