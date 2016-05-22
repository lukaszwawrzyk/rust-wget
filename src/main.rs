#![feature(advanced_slice_patterns, slice_patterns)]

extern crate regex;
extern crate url;
extern crate time;
extern crate getopts;
extern crate rpassword;
extern crate hyper;

#[macro_use]
mod common;
mod request;
mod response;
mod progress;
mod options;
mod http;
mod path_resolve;

use options::Options;
use std::env;
use http::Http;

fn main() {
  let args: Vec<String> = env::args().collect();
  let options = Options::retreive(args);

  let result = options.and_then(|opts| {
    let http = Http::new(opts);
    http.download_all()
  });

  if let Err(e) = result {
    println!("\n{}", e);
  }
}
