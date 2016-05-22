use getopts::ParsingStyle;
use hyper::Url;
use common::{CompoundResult, CompoundError};
use rpassword;
use getopts;
use std::io;
use std::io::Write;

pub struct Options {
  pub continue_download: bool,
  pub show_response: bool,
  pub tries: Option<u64>,
  pub timeout_secs: Option<u64>,
  pub backup_limit: Option<u64>,
  pub credentials: Option<Credentials>,
  pub headers: Vec<String>,
  pub urls: Vec<Url>,
}

pub struct Credentials {
  pub user: String,
  pub password: String,
}

impl Credentials {
  pub fn new(user: String, password: String) -> Credentials {
    Credentials {
      user: user,
      password: password,
    }
  }
}

impl Options {
  pub fn retreive(raw_params: Vec<String>) -> CompoundResult<Options> {
    let program = raw_params[0].clone();
    let options_parser = Self::config_parser();

    let parsed_opts = match options_parser.parse(&raw_params[1..]) {
      Ok(matches) => matches,
      Err(_) => fail!(CompoundError::UserError(Self::usage(&program, options_parser))),
    };

    // show help
    if parsed_opts.opt_present("h") {
      fail!(CompoundError::UserError(Self::usage(&program, options_parser)));
    }

    let mut options = Self::default_options();

    // continue
    options.continue_download = parsed_opts.opt_present("c");

    // show response
    options.show_response = parsed_opts.opt_present("S");

    // tries
    if let Some(tries_str) = parsed_opts.opt_str("t") {
      options.tries = if tries_str == "inf" || tries_str == "0" {
        None
      } else {
        tries_str.parse::<u64>().ok().or(options.tries)
      }
    };

    // timeout
    if let Some(timeout_str) = parsed_opts.opt_str("T") {
      options.timeout_secs = timeout_str.parse::<u64>().ok()
        .or(options.timeout_secs)
        .and_then(|secs| if secs == 0 { None } else { Some(secs) })
    };

    // backups
    if let Some(backups_str) = parsed_opts.opt_str("backups") {
      options.backup_limit = backups_str.parse::<u64>().ok()
        .or(options.backup_limit)
        .and_then(|limit| if limit == 0 { None } else { Some(limit) })
    };

    // credentials
    options.credentials = match (parsed_opts.opt_str("user"), parsed_opts.opt_str("password"), parsed_opts.opt_present("ask-password")) {
      (Some(login), Some(password), false) => Some(Credentials::new(login, password)),
      (Some(login), None, true) => {
        let password = try!(Self::read_password_from_user());
        Some(Credentials::new(login, password))
      },
      (Some(login), None, false) => Some(Credentials::new(login, "".to_string())),
      (None, Some(password), false) => Some(Credentials::new("".to_string(), password)),
      (None, None, true) => {
        let password = try!(Self::read_password_from_user());
        Some(Credentials::new("".to_string(), password))
      },
      (None, None, false) => None,
      _ => fail!(CompoundError::UserError(Self::usage(&program, options_parser))),
    };

    // headers
    options.headers = parsed_opts.opt_strs("header");

    // urls
    let urls_res: CompoundResult<Vec<Url>> = parsed_opts.free.into_iter()
      .map(|ref url_str| Url::parse(url_str)
        .map_err(|_| CompoundError::UserError(format!("Invalid url {}", url_str).to_string())))
      .collect();

    let urls = try!(urls_res);

    if urls.is_empty() {
      fail!(CompoundError::UserError(Self::usage(&program, options_parser)));
    } else {
      options.urls = urls;
    }

    Ok(options)
  }

  fn default_options() -> Options {
    Options {
      continue_download: false,
      show_response: false,
      tries: Some(20),
      timeout_secs: Some(900),
      backup_limit: None,
      credentials: None,
      headers: Vec::new(),
      urls: Vec::new(),
    }
  }

  fn read_password_from_user() -> CompoundResult<String> {
    print!("Password: ");
    let _ = io::stdout().flush();
    Ok(try!(rpassword::read_password()))
  }

  fn config_parser() -> getopts::Options {
    let mut opts = getopts::Options::new();
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

    return opts;
  }

  fn usage(program: &str, opts: getopts::Options) -> String {
    let brief = format!("Usage: {} [options] URL", program);
    return opts.usage(&brief).to_string();
  }
}
