#![feature(advanced_slice_patterns, slice_patterns)]

extern crate url;
extern crate time;
extern crate getopts;

#[macro_use]
mod common;
mod request;
mod response;
mod progress;

use getopts::Options;
use getopts::ParsingStyle;
use std::env;
use response::Response;
use request::Request;
use common::Result;
use std::path::Path;
use std::net::TcpStream;
use url::Url;
use std::result;



const DEFAULT_FILE_NAME: &'static str = "out";

fn download(source_url: &str) -> Result<String> {
  let url = try_str!(Url::parse(source_url));

  let mut socket = try!(connect(&url));

  let file_name = file_name_from_url(&url);
  let destination_path = Path::new(&file_name);


  let request = try!(Request::format(&url));
  try!(request.send(&mut socket));

  let mut response = Response::new(socket);

  return response.download(&destination_path).map(|_| format!("Downloaded to {}", destination_path.to_string_lossy()).to_string());


  fn connect(url: &Url) -> Result<TcpStream> {
    fn default_port(url: &Url) -> result::Result<u16, ()> {
      match url.scheme() {
        "http" => Ok(80),
        _ => Err(()),
      }
    }

    let socket = url.with_default_port(default_port).and_then(TcpStream::connect);

    str_err!(socket)
  }

  fn file_name_from_url(url: &Url) -> String {
    url.path_segments()
      .and_then(|segments| segments.last())
      .map(|s| s.to_string())
      .and_then(|s| if s.is_empty() { None } else { Some(s) })
      .unwrap_or(DEFAULT_FILE_NAME.to_string())
  }
}

fn print_usage(program: &str, opts: Options) {
    let brief = format!("Usage: {} [options] URL", program);
    print!("{}", opts.usage(&brief));
}

fn main() {
  let args: Vec<String> = env::args().collect();
  let program = args[0].clone();
  let mut opts = Options::new();
  opts.parsing_style(ParsingStyle::FloatingFrees);
  opts.optflag("h", "help", "Show this help menu");
  opts.optflag("c", "continue", "Continue getting a partially-downloaded file");
  opts.optflag("S", "server-response", "Print the headers sent by HTTP servers");
  opts.optflag("", "ask-password", "Prompt for a password for each connection established");
  opts.optopt("t", "tries", "Set number of tries to number. Specify 0 for infinite retrying.", "NUMBER");
  opts.optopt("T", "timeout", "Set the network timeout to seconds seconds", "SECONDS");
  opts.optopt("", "backups", "Before (over)writing a file, back up an existing file by adding a .1 suffix to the file name. Such backup files are rotated to .2, .3, and so on, up to backups (and lost beyond that).", "BACKUPS");
  opts.optopt("", "user", "Specify the username for HTTP file retrieval", "USER");
  opts.optopt("", "password", "Specify the password for HTTP file retrieval", "PASSWORD");
  opts.optmulti("", "header", "Send header-line along with the rest of the headers in each HTTP request", "HEADER-LINE");

  let matches = match opts.parse(&args[1..]) {
    Ok(m) => m,
    Err(err) => return print_usage(&program, opts),
  };

  if matches.opt_present("h") || matches.free.is_empty() {
    return print_usage(&program, opts);
  }

  let result = match &matches.free[..] {
    [ref source_url] =>
      download(source_url),
    _ =>
      Err("Multiple files are not supported".to_owned()), // TODO yet
  };

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

--backups=backups
    Before (over)writing a file, back up an existing file by adding a .1 suffix (_1 on VMS) to the
    file name.  Such backup files are rotated to .2, .3, and so on, up to backups (and lost beyond
    that).

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

 -S
 --server-response
   Print the headers sent by HTTP servers and responses sent by FTP servers.

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

--user=user
--password=password
   Specify the username user and password password for both FTP and HTTP file retrieval.  These
   parameters can be overridden using the --ftp-user and --ftp-password options for FTP connections
   and the --http-user and --http-password options for HTTP connections.

--ask-password
   Prompt for a password for each connection established. Cannot be specified when --password is
   being used, because they are mutually exclusive.


--header=header-line
   Send header-line along with the rest of the headers in each HTTP request.  The supplied header
   is sent as-is, which means it must contain name and value separated by colon, and must not
   contain newlines.

   You may define more than one additional header by specifying --header more than once.

           wget --header='Accept-Charset: iso-8859-2' \
                --header='Accept-Language: hr'        \
                  http://fly.srk.fer.hr/

   Specification of an empty string as the header value will clear all previous user-defined
   headers.

   As of Wget 1.10, this option can be used to override headers otherwise generated automatically.
   This example instructs Wget to connect to localhost, but to specify foo.bar in the "Host"
   header:

           wget --header="Host: foo.bar" http://localhost/

   In versions of Wget prior to 1.10 such use of --header caused sending of duplicate headers.

HTTPS

logi:

Translacja fly.srk.fer.hr (fly.srk.fer.hr)... 80.241.220.122
Łączenie się z fly.srk.fer.hr (fly.srk.fer.hr)|80.241.220.122|:80... połączono.
Żądanie HTTP wysłano, oczekiwanie na odpowiedź... 200 OK
Długość: 1410 (1,4K) [text/html]
Zapis do: `fly.srk.fer.hr/index.html'

     0K .                                                     100%  214M=0s


*/
