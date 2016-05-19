#![feature(advanced_slice_patterns, slice_patterns)]

extern crate regex;
extern crate url;
extern crate time;
extern crate getopts;
extern crate rpassword;
extern crate rustc_serialize;

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
// TODO follow redirects
// TODO show progress in %, kb of all, speed
// TODO check status code to see if should look for eof or abort
// TODO continue ++++++ on progress bar
// TODO if range request is sent to server with chunked encoding it will send 200 + Content-Length 0 but no chunked header but message still will be chunked - handle this

/*
-t number
--tries=number
    Set number of tries to number. Specify 0 or inf for infinite retrying.  The default is to retry
    20 times, with the exception of fatal errors like "connection refused" or "not found" (404),
    which are not retried.


-T seconds
--timeout=seconds
   Set the network timeout to seconds seconds.  This is equivalent to specifying --dns-timeout,
   --connect-timeout, and --read-timeout, all at the same time.

   When interacting with the network, Wget can check for timeout and abort the operation if it
   takes too long.  This prevents anomalies like hanging reads and infinite connects.  The only
   timeout enabled by default is a 900-second read timeout.  Setting a timeout to 0 disables it
   altogether.  Unless you know what you are doing, it is best not to change the default timeout
   settings.

   All timeout-related options accept decimal values, as well as subsecond values.  For example,
   0.1 seconds is a legal (though unwise) choice of timeout.  Subsecond timeouts are useful for
   checking server response times or for testing network latency.

HTTPS

logi:

Translacja fly.srk.fer.hr (fly.srk.fer.hr)... 80.241.220.122
Łączenie się z fly.srk.fer.hr (fly.srk.fer.hr)|80.241.220.122|:80... połączono.
Żądanie HTTP wysłano, oczekiwanie na odpowiedź... 200 OK
Długość: 1410 (1,4K) [text/html]
Zapis do: `fly.srk.fer.hr/index.html'

     0K .                                                     100%  214M=0s


*/
