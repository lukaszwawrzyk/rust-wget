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
use common::Result;
use std::result;
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

/*
-t number
--tries=number
    Set number of tries to number. Specify 0 or inf for infinite retrying.  The default is to retry
    20 times, with the exception of fatal errors like "connection refused" or "not found" (404),
    which are not retried.
-c
--continue
    Continue getting a partially-downloaded file.  This is useful when you want to finish up a
    download started by a previous instance of Wget, or by another program.  For instance:

            wget -c ftp://sunsite.doc.ic.ac.uk/ls-lR.Z

    If there is a file named ls-lR.Z in the current directory, Wget will assume that it is the first
    portion of the remote file, and will ask the server to continue the retrieval from an offset
    equal to the length of the local file.

    Note that you don't need to specify this option if you just want the current invocation of Wget
    to retry downloading a file should the connection be lost midway through.  This is the default
    behavior.  -c only affects resumption of downloads started prior to this invocation of Wget, and
    whose local files are still sitting around.

    Without -c, the previous example would just download the remote file to ls-lR.Z.1, leaving the
    truncated ls-lR.Z file alone.

    Beginning with Wget 1.7, if you use -c on a non-empty file, and it turns out that the server
     does not support continued downloading, Wget will refuse to start the download from scratch,
     which would effectively ruin existing contents.  If you really want the download to start from
     scratch, remove the file.

     Also beginning with Wget 1.7, if you use -c on a file which is of equal size as the one on the
     server, Wget will refuse to download the file and print an explanatory message.  The same
     happens when the file is smaller on the server than locally (presumably because it was changed
     on the server since your last download attempt)---because "continuing" is not meaningful, no
     download occurs.

     On the other side of the coin, while using -c, any file that's bigger on the server than locally
     will be considered an incomplete download and only "(length(remote) - length(local))" bytes will
     be downloaded and tacked onto the end of the local file.  This behavior can be desirable in
     certain cases---for instance, you can use wget -c to download just the new portion that's been
     appended to a data collection or log file.

     However, if the file is bigger on the server because it's been changed, as opposed to just
     appended to, you'll end up with a garbled file.  Wget has no way of verifying that the local
     file is really a valid prefix of the remote file.  You need to be especially careful of this
     when using -c in conjunction with -r, since every file will be considered as an "incomplete
     download" candidate.

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
