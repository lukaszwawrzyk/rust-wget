extern crate url;

use common::CompoundResult;
use hyper::Url;
use options::Options;
use common;
use hyper::header::Headers;
use hyper::header;
use hyper::mime;
use hyper::client::response::Response;
use std::time::Duration;
use hyper::client::{Client, RedirectPolicy};

pub struct Request {
}

impl Request {
  pub fn send_default(url: &Url, options: &Options) -> CompoundResult<Response> {
    Self::build_and_send(url, options, None)
  }

  pub fn send_with_range_from(url: &Url, options: &Options, range_from: u64) -> CompoundResult<Response> {
    Self::build_and_send(url, options, Some(header::Range::Bytes(vec![header::ByteRangeSpec::AllFrom(range_from)])))
  }

  fn build_and_send(url: &Url, options: &Options, special_header: Option<header::Range>) -> CompoundResult<Response> {
    let mut headers = Headers::new();
    headers.set(header::Accept(vec![header::qitem(mime::Mime(mime::TopLevel::Star, mime::SubLevel::Star, vec![]))]));
    if let Some(ref credentials) = options.credentials {
      headers.set(
        header::Authorization(
          header::Basic {
            username: credentials.user.clone(),
            password: Some(credentials.password.clone())
          }
        )
      )
    }

    special_header.map(|header| headers.set(header));

    // headers from user that may override current
    let extra_headers_raw = common::parse_header_lines(&options.headers[..]);
    for (k, v) in extra_headers_raw {
      headers.set_raw(k, vec![v.into_bytes()]);
    }

    let client = Self::create_client(options);
    let request = client.get(url.clone()).headers(headers);

    Ok(try!(request.send()))
  }

  fn create_client(options: &Options) -> Client {
    let timeout = options.timeout_secs.map(Duration::from_secs);
    let mut client = Client::new();
    client.set_redirect_policy(RedirectPolicy::FollowNone);
    client.set_read_timeout(timeout);
    client.set_write_timeout(timeout);
    client
  }
}
