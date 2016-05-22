use error::CompoundResult;
use std::io;
use std::io::{Read, Write};
use progress::Progress;

const BUFFER_SIZE: usize = 16 * 1024;

pub struct ResponseBuffer {
  buffer: [u8; BUFFER_SIZE],
}

impl ResponseBuffer {
  fn new() -> ResponseBuffer {
    ResponseBuffer {
      buffer: [0; BUFFER_SIZE],
    }
  }

  pub fn read_chunked<R: Read>(source: &mut R, destination: &mut Write, progress: &mut Progress) -> CompoundResult<()> {
    let mut buffer = Self::new();
    buffer.read_bytes(source, None, destination, progress)
  }

  pub fn read_fixed_bytes<R: Read>(source: &mut R, expected_length: u64, destination: &mut Write, progress: &mut Progress) -> CompoundResult<()> {
    let mut buffer = Self::new();
    buffer.read_bytes(source, Some(expected_length), destination, progress)
  }

  fn read_bytes<R: Read>(&mut self, source: &mut R, expected_length_opt: Option<u64>, destination: &mut Write, progress: &mut Progress) -> CompoundResult<()> {
    let mut total_bytes_read: u64 = 0;
    loop {
      let bytes_read: usize = match expected_length_opt {
        Some(expected_length) => {
          let bytes_left = expected_length - total_bytes_read;
          try!(source.by_ref().take(bytes_left).read(&mut self.buffer[..]))
        },
        None => try!(source.read(&mut self.buffer[..])),
      };

      if bytes_read == 0 {
        if expected_length_opt.map_or(false, |expected_length| total_bytes_read != expected_length) {
          fail!(io::Error::new(io::ErrorKind::UnexpectedEof, format!("Failed to read expected number of bytes. Read {} of {}", total_bytes_read, expected_length_opt.unwrap()).to_string()));
        } else {
          return Ok(());
        }
      }

      try!(destination.write_all(&self.buffer[0..bytes_read]));

      progress.update(bytes_read as u64);

      total_bytes_read += bytes_read as u64;

      if expected_length_opt.map_or(false, |expected_length| total_bytes_read >= expected_length) {
        return Ok(());
      }
    }
  }
}
