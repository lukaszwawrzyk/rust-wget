use std::fs;
use std::path::{Path, PathBuf};
use hyper::Url;
use options::Options;
use error::CompoundResult;
use std::io;

const DEFAULT_FILE_NAME: &'static str = "out";

pub fn get_destination_path(url: &Url, options: &Options) -> CompoundResult<PathBuf> {
  let basic_file_name = file_name_from_url(url);
  if should_continue_download(&basic_file_name, options) {
    Ok(PathBuf::from(&basic_file_name))
  } else {
    Ok(PathBuf::from(try!(backup_file_name(&basic_file_name, options))))
  }
}

fn file_name_from_url(url: &Url) -> String {
  url.path_segments()
    .and_then(|segments| segments.last())
    .map(|s| s.to_string())
    .and_then(|s| if s.is_empty() { None } else { Some(s) })
    .unwrap_or(DEFAULT_FILE_NAME.to_string())
}

fn should_continue_download(file_name: &str, options: &Options) -> bool {
  let file_metadata = fs::metadata(Path::new(file_name));

  options.continue_download && file_metadata.is_ok()
}

fn backup_file_name(basic_name: &str, options: &Options) -> CompoundResult<String> {
  let dir = try!(fs::read_dir(Path::new("./")));
  let files: Vec<String> = dir
    .flat_map(|r| r.ok())
    .flat_map(|entry| entry.file_name().to_str().map(|s| s.to_string()))
    .collect::<Vec<String>>();
  if !files.contains(&basic_name.to_string()) {
    return Ok(basic_name.to_string());
  }

  let prefix: &str = &format!("{}.", basic_name);
  let mut current_indices: Vec<u64> = files.iter()
    .filter(|s| s.starts_with(prefix))
    .map(|s| (&s[(basic_name.len() + 1)..]).to_string())
    .flat_map(|s| s.parse::<u64>().ok())
    .collect();
  current_indices.sort();

  match options.backup_limit {
    None => {
      let next_index = (1..).zip(current_indices.iter())
      .find(|&(expected_index, &actual_index)| expected_index < actual_index)
      .map(|(free_index, _)| free_index)
      .unwrap_or(current_indices.len() as u64 + 1);

      Ok(format!("{}.{}", basic_name, next_index).to_string())
    },
    Some(limit) => {
      current_indices.push(limit + 1);
      let missing_index = (1..(limit + 1)).zip(current_indices.iter())
      .find(|&(expected_index, &actual_index)| expected_index < actual_index)
      .map(|(free_index, _)| free_index);

      match missing_index {
        Some(next_index) => Ok(format!("{}.{}", basic_name, next_index).to_string()),
        None => {
          try!(shift_names(basic_name, limit));
          Ok(basic_name.to_string())
        },
      }
    },
  }
}

fn shift_names(basic_name: &str, limit: u64) -> io::Result<()> {
  try!(fs::remove_file(to_path(basic_name, limit)));
  for i in (1..limit).rev() {
    try!(fs::rename(to_path(basic_name, i), to_path(basic_name, i + 1)));
  }
  try!(fs::rename(Path::new(basic_name), to_path(basic_name, 1)));
  return Ok(());

  fn to_path(basic_name: &str, num: u64) -> PathBuf {
    let name = format!("{}.{}", basic_name, num);
    PathBuf::from(name)
  }
}
