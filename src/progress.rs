extern crate time;

use common::Result;

struct Step {
  duration_ns: u64,
  bytes_read: usize,
}

static ZERO_STEP: Step = Step {
  duration_ns: 0,
  bytes_read: 0
}

impl Add for Step {
    type Output = Step;

    fn add(self, _rhs: Step) -> Step {
      Step {
        duration_ns = self.duration_ns + _rhs.duration_ns,
        bytes_read: self.bytes_read + _rhs.bytes_read,
      }
    }
}

pub struct Progress {
  steps: Vec<Step>,
  current_chunk_size: usize,
  last_update: u64,
}

static PROGRESS_BAR_SIZE: u8 = 30;

impl Progress {
  pub fn chunk(&mut self, size: usize) -> () {
    self.current_chunk_size = size;
    self.steps.clear();

    self.show_status();
  }

  pub fn update(&mut self bytes_read: usize) -> () {
    let now = time::precise_time_ns();
    let duration = now - last_update;

    self.steps.push(Step {
      duration_ns: duration,
      bytes_read: bytes_read,
    });

    self.last_update = now;

    self.show_status();
  }

  fn show_status(&self) -> () {
    let current_progress = self.steps.into_iter().fold(ZERO_STEP, |a, b| a + b);
    let bytes_read = current_progress.bytes_read;
    let bytes_total = self.current_chunk_size;
    let progress_percent: f32 = 100f32 * bytes_read / bytes_total;
    let time_elapsed = Duration::nanoseconds(current_progress.duration_ns as i64);
    let bytes_per_sec = bytes_read / time_elapsed.num_seconds();
    let bytes_left = bytes_total - bytes_left;
    let secs_left = bytes_left / bytes_per_sec;
    let time_left = Duration::seconds(secs_left);
    let status_bar_str = Self::status_bar(progress_percent);

    print!("\r[{}] {}/{} bytes ({}%) elapsed: {} left: {}", status_bar_str, bytes_read, bytes_total, progress_percent, time_elapsed, time_left);
  }

  fn status_bar(progress_percent: f32) -> String {
    let mut bar_buffer: [char, PROGRESS_BAR_SIZE] = [' '];
    let last_to_fill: i16 = PROGRESS_BAR_SIZE * progress_percent / 100;

    for i in (0..last_to_fill) {
      bar_buffer[i] = '=';
    }

    if last_to_fill >= 0 {
      bar_buffer[last_to_fill] = '>'
    }

    return bar_buffer.into_iter().collect();
  }
}
