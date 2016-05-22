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

  match result {
    Ok(msg) => println!("\n{}", msg),
    Err(e) => println!("\n{}", e),
  }
}

// TODO check https
// TODO show progress in %, kb of all, speed
// TODO if range request is sent to server with chunked encoding it will send 200 + Content-Length 0 but no chunked header but message still will be chunked - handle this

/*
HTTPS

logi:

Translacja fly.srk.fer.hr (fly.srk.fer.hr)... 80.241.220.122
Łączenie się z fly.srk.fer.hr (fly.srk.fer.hr)|80.241.220.122|:80... połączono.
Żądanie HTTP wysłano, oczekiwanie na odpowiedź... 200 OK
Długość: 1410 (1,4K) [text/html]
Zapis do: `fly.srk.fer.hr/index.html'

     0K .                                                     100%  214M=0s


*/
