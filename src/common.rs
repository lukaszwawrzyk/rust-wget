use std::result;
use std::collections::HashMap;

pub type Result<T> = result::Result<T, String>;

#[macro_export]
macro_rules! str_err {
  ($e:expr) => {
    $e.map_err(|err| err.to_string());
  };
}

#[macro_export]
macro_rules! try_str {
  ($e:expr) => {
    try!($e.map_err(|err| err.to_string()));
  };
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
