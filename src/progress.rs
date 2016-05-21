use std::ops::Add;
use time::Duration;
use time::precise_time_ns;
use std::cmp;

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
  total_size: Option<u64>,
  predownloaded_size: Option<u64>,
  last_update: u64,
  initialized: bool,
}

const PROGRESS_BAR_SIZE: usize = 30;

impl Progress {
  pub fn new() -> Progress {
    Progress {
      steps: Vec::new(),
      total_size: None,
      predownloaded_size: None,
      last_update: 0,
      initialized: false,
    }
  }

  pub fn try_set_predownloaded(&mut self, predownloaded_size: u64) -> () {
    if self.predownloaded_size.is_none() {
      self.predownloaded_size = Some(predownloaded_size);
    }
  }

  pub fn try_initialize_indeterminate(&mut self) -> () {
    self.try_initialize(None);
  }

  pub fn try_initialize_sized(&mut self, size_to_download: u64) -> () {
    let predownloaded_size = self.predownloaded_size.unwrap_or(0);
    self.try_initialize(Some(size_to_download + predownloaded_size));
  }

  fn try_initialize(&mut self, total_size: Option<u64>) -> () {
    if !self.initialized {
      self.total_size = total_size;

      println!("");
      self.show_status();

      self.initialized = true;
    }

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
    match self.total_size {
      Some(total_size) => {
        let current_progress = self.steps.iter().fold(ZERO_STEP, |acc, el| &acc + el);

        let bytes_read = current_progress.bytes_read;
        let bytes_total = total_size;
        let bytes_left = bytes_total - bytes_read - self.predownloaded_size.unwrap_or(0);

        let new_progress_percent: f32 = 100f32 * safe_div_f32!(bytes_read as f32, bytes_total as f32);

        let time_elapsed = Duration::nanoseconds(current_progress.duration_ns as i64);

        let bytes_per_sec = safe_div_u64!(bytes_read, time_elapsed.num_seconds() as u64);

        let secs_left = safe_div_u64!(bytes_left, bytes_per_sec);
        let time_left = Duration::seconds(secs_left as i64);

        let status_bar_str = self.progress_bar(new_progress_percent);

        print!("\r[{}] {}/{} bytes ({}%) {}B/s elapsed: {} left: {}", status_bar_str, bytes_read + self.predownloaded_size.unwrap_or(0), bytes_total, new_progress_percent, bytes_per_sec, time_elapsed, time_left);
      },
      None => {
        let current_progress = self.steps.iter().fold(ZERO_STEP, |acc, el| &acc + el);

        let bytes_read = current_progress.bytes_read;
        let time_elapsed = Duration::nanoseconds(current_progress.duration_ns as i64);
        let bytes_per_sec = safe_div_u64!(bytes_read, time_elapsed.num_seconds() as u64);

        let status_bar_str = self.status_bar(time_elapsed.num_seconds() as u64);

        print!("\r[{}] {}/? bytes {}B/s elapsed: {}", status_bar_str, bytes_read, bytes_per_sec, time_elapsed);
      }
    }
  }


  fn status_bar(&self, time_elapsed: u64) -> String {
    let mut bar_buffer: [char; PROGRESS_BAR_SIZE] = [' '; PROGRESS_BAR_SIZE];
    let indicator_center_position = cmp::min(((time_elapsed as i64 % (PROGRESS_BAR_SIZE as i64 * 2)) - PROGRESS_BAR_SIZE as i64 + 1).abs() as usize, PROGRESS_BAR_SIZE - 1);

    bar_buffer[indicator_center_position as usize] = '+';
    if indicator_center_position as i32 - 1 >= 0 {
      bar_buffer[indicator_center_position as usize - 1] = '<'
    }

    if indicator_center_position + 1 < PROGRESS_BAR_SIZE {
      bar_buffer[indicator_center_position as usize + 1] = '>'
    }

    return bar_buffer.into_iter().cloned().collect();
  }

  fn progress_bar(&self, progress_percent: f32) -> String {
    let mut bar_buffer: [char; PROGRESS_BAR_SIZE] = [' '; PROGRESS_BAR_SIZE];
    let predownloaded_percent = 100f32 * safe_div_f32!(self.predownloaded_size.unwrap_or(0) as f32, self.total_size.unwrap() as f32);
    let last_to_fill_predownloaded: i16 = (PROGRESS_BAR_SIZE as f32 * predownloaded_percent / 100f32) as i16 - 1;
    let last_to_fill_new_progress: i16 = (PROGRESS_BAR_SIZE as f32 * progress_percent / 100f32) as i16 - 1;

    for i in 0..last_to_fill_predownloaded + 1 {
      bar_buffer[i as usize] = '+';
    }

    let (new_fill_start, new_fill_end) = if last_to_fill_predownloaded >= 0 && last_to_fill_new_progress >= 0 {
      (last_to_fill_predownloaded, last_to_fill_predownloaded + last_to_fill_new_progress + 1)
    } else {
      (0, last_to_fill_new_progress)
    };

    for i in new_fill_start..new_fill_end {
      bar_buffer[i as usize] = '=';
    }

    if new_fill_end >= 0 {
      bar_buffer[new_fill_end as usize] = '>'
    }

    return bar_buffer.into_iter().cloned().collect();
  }
}
