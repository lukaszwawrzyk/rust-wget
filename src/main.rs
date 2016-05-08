extern crate url;

use std::io;
use std::io::BufReader;
use std::io::prelude::*;
use std::fs::File;
use std::path::Path;
use std::net::TcpStream;
use url::Url;
use std::result;

type Result<T> = result::Result<T, String>;

fn connect(url: &Url) -> io::Result<TcpStream> {
    TcpStream::connect(try!(url.with_default_port(default_port)))
}

fn default_port(url: &Url) -> result::Result<u16, ()> {
    match url.scheme() {
        "https" => Ok(443),
        "http" => Ok(80),
        _ => Err(()),
    }
}

fn format_request(url: &Url) -> Result<String> {
    url.host_str().map(|host| format!(
        "GET {} HTTP/1.1\r\nHost: {}\r\nAccept: */*\r\n\r\n",
        url.path(), host
    )).ok_or("No host found in url".to_owned())
}

macro_rules! str_err {
    ($e:expr) => {
        $e.map_err(|err| err.to_string());
    };
}

fn send_request(mut socket: &TcpStream, request: &String) -> Result<()> {
    str_err!(socket.write(request.as_bytes()).map(|_| ()))
}

fn read_raw_headers(socket: &TcpStream) -> Result<Vec<String>> {
    let response = BufReader::new(socket);

    let headers: io::Result<Vec<String>> = response.lines()
        .take_while(|res| {
            match *res {
                Ok(ref line) if !line.is_empty() => true,
                _ => false
            }
        }).collect();

    str_err!(headers)
}

fn main() {
    let source_url = "http://google.com/";

    let result= str_err!(Url::parse(source_url)).and_then(|url|
                str_err!(connect(&url)).and_then(|socket|
                format_request(&url).and_then(|request|
                send_request(&socket, &request).and(
                read_raw_headers(&socket)))));

    match result {
        Ok(headers) => println!("{:?}", headers),
        Err(e) => println!("{}", e),
    }
}
