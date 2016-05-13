use std::ops::Add;
use common::Result;
use time::Duration;
use time::precise_time_ns;

macro_rules! safe_div_f32 {
  ($num:expr, $denom:expr) => {
    if $denom == 0f32 {
      0f32
    } else {
      $num / $denom
    }
  };
}

macro_rules! safe_div_u64 {
  ($num:expr, $denom:expr) => {
    if $denom == 0u64 {
      0u64
    } else {
      $num / $denom
    }
  };
}

struct Step {
  duration_ns: u64,
  bytes_read: u64,
}

const ZERO_STEP: Step = Step {
  duration_ns: 0,
  bytes_read: 0
};

impl<'a> Add for &'a Step {
    type Output = Step;

    fn add(self, _rhs: &Step) -> Step {
      Step {
        duration_ns: self.duration_ns + _rhs.duration_ns,
        bytes_read: self.bytes_read + _rhs.bytes_read,
      }
    }
}

pub struct Progress {
  steps: Vec<Step>,
  current_chunk_size: u64,
  last_update: u64,
}

const PROGRESS_BAR_SIZE: usize = 30;

impl Progress {
  pub fn new() -> Progress {
    Progress {
      steps: Vec::new(),
      current_chunk_size: 0,
      last_update: 0,
    }
  }

  pub fn chunk(&mut self, size: u64) -> () {
    self.current_chunk_size = size;
    self.steps.clear();

    println!("");
    self.show_status();
  }

  pub fn update(&mut self, bytes_read: u64) -> () {
    let now = precise_time_ns();
    let duration = now - self.last_update;

    self.steps.push(Step {
      duration_ns: duration,
      bytes_read: bytes_read,
    });

    self.last_update = now;

    self.show_status();
  }

  fn show_status(&self) -> () {
    let current_progress = self.steps.iter().fold(ZERO_STEP, |acc, el| &acc + el);

    let bytes_read = current_progress.bytes_read;
    let bytes_total = self.current_chunk_size;
    let bytes_left = bytes_total - bytes_read;

    let progress_percent: f32 = 100f32 * safe_div_f32!(bytes_read as f32, bytes_total as f32);

    let time_elapsed = Duration::nanoseconds(current_progress.duration_ns as i64);

    let bytes_per_sec = safe_div_u64!(bytes_read, time_elapsed.num_seconds() as u64);

    let secs_left = safe_div_u64!(bytes_left, bytes_per_sec);
    let time_left = Duration::seconds(secs_left as i64);

    let status_bar_str = Self::status_bar(progress_percent);

    print!("\r[{}] {}/{} bytes ({}%) {}B/s elapsed: {} left: {}", status_bar_str, bytes_read, bytes_total, progress_percent, bytes_per_sec, time_elapsed, time_left);
  }

  fn status_bar(progress_percent: f32) -> String {
    let mut bar_buffer: [char; PROGRESS_BAR_SIZE] = [' '; PROGRESS_BAR_SIZE];
    let last_to_fill: i16 = (PROGRESS_BAR_SIZE as f32 * progress_percent / 100f32) as i16 - 1;

    for i in 0..last_to_fill {
      bar_buffer[i as usize] = '=';
    }

    if last_to_fill >= 0 {
      bar_buffer[last_to_fill as usize] = '>'
    }

    return bar_buffer.into_iter().cloned().collect();
  }
}
