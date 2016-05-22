use std::result;
use std::collections::HashMap;
use std::io;
use std::io::ErrorKind::*;
use std::convert;
use std::fmt;
use std::error::Error;
use hyper;

pub type CompoundResult<T> = result::Result<T, CompoundError>;

#[derive(Debug)]
pub enum CompoundError {
  UserError(String),
  TemporaryServerError,
  BadResponse(String),
  UnsupportedResponse,
  ServerDoesNotSupportContinuation,
  ConnectionError(io::Error),
  IoError(io::Error),
  OtherError(String),
}

impl Error for CompoundError {
  fn description(&self) -> &str {
    match *self {
      CompoundError::TemporaryServerError => "temporary server error",
      CompoundError::UnsupportedResponse => "unsupported response",
      CompoundError::ServerDoesNotSupportContinuation => "server does not support range header",
      CompoundError::UserError(_) => "user error",
      CompoundError::BadResponse(_) => "bad response",
      CompoundError::ConnectionError(_) => "connection error",
      CompoundError::IoError(_) => "io error",
      CompoundError::OtherError(_) => "other error",
    }
  }

  fn cause(&self) -> Option<&Error> {
    match *self {
      CompoundError::ConnectionError(ref err) => Some(err as &Error),
      CompoundError::IoError(ref err) => Some(err as &Error),
      _ => None,
    }
  }
}

impl fmt::Display for CompoundError {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    let desc = match *self {
      CompoundError::TemporaryServerError => "Temporary server error (5xx)".to_string(),
      CompoundError::UnsupportedResponse => "Unsupported response. Supported response must be either chunked or have Content-Length".to_string(),
      CompoundError::ServerDoesNotSupportContinuation => "Server does not support range header".to_string(),
      CompoundError::UserError(ref msg) => format!("{}", msg).to_string(),
      CompoundError::BadResponse(ref msg) => format!("Bad response: {}", msg).to_string(),
      CompoundError::ConnectionError(ref err) => format!("Connection error: {}", err.to_string()).to_string(),
      CompoundError::IoError(ref err) => format!("IO Error: {}", err.to_string()).to_string(),
      CompoundError::OtherError(ref msg) => format!("Error: {}", msg).to_string(),
    };
    write!(f, "{}", desc)
  }
}

impl convert::From<hyper::error::Error> for CompoundError {
  fn from(err: hyper::error::Error) -> CompoundError {
    match err {
      hyper::error::Error::Io(e) =>
        convert::From::from(e),
      hyper::error::Error::Uri(e) =>
        CompoundError::UserError(format!("Invalid url ({})", e).to_string()),
      e =>
        CompoundError::BadResponse(format!("{}", e).to_string()),
    }
  }
}

impl convert::From<io::Error> for CompoundError {
  fn from(err: io::Error) -> CompoundError {
    match err.kind() {
      ConnectionRefused | ConnectionReset | ConnectionAborted | NotConnected | AddrInUse | AddrNotAvailable | TimedOut | Interrupted =>
        CompoundError::ConnectionError(err),
      _ =>
        CompoundError::IoError(err),
    }
  }
}

impl convert::From<String> for CompoundError {
  fn from(err: String) -> CompoundError {
    CompoundError::OtherError(err)
  }
}

#[macro_export]
macro_rules! fail {
  ($err:expr) => (
    return ::std::result::Result::Err(::std::convert::From::from($err));
  )
}


// TODO move it somewhere
pub fn parse_header_lines(header_lines: &[String]) -> HashMap<String, String> {
  header_lines.into_iter().flat_map(|line| {
    let splitted: Vec<&str> = line.split(": ").collect();
    match &splitted[..] {
      [key, value] => Some((key.to_string(), value.to_string())),
      _ => None
    }
  }).collect()
}
