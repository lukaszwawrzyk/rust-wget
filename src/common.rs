use std::result;

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
