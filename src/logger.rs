
use std::time::{self, SystemTime, UNIX_EPOCH};
use std::io::{BufWriter, Write};

struct SimpleTime;
impl SimpleTime {
  fn epoch_to_ymdhms(seconds: u64) -> (i32, u32, u32, u32, u32, u32) {
    // Constants for time calculations
    const SECONDS_IN_MINUTE: u64 = 60;
    const SECONDS_IN_HOUR: u64 = 3600;
    const SECONDS_IN_DAY: u64 = 86400;

    // Leap year and normal year days
    const DAYS_IN_YEAR: [u32; 2] = [365, 366];
    const DAYS_IN_MONTH: [[u32; 12]; 2] = [
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31], // Normal years
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]  // Leap years
    ];

    // Calculate the number of days since the epoch
    let mut remaining_days = seconds / SECONDS_IN_DAY;

    // Determine the current year
    let mut year = 1970;
    loop {
        let leap_year = if (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0) { 1 } else { 0 };
        if remaining_days < DAYS_IN_YEAR[leap_year] as u64 {
            break;
        }
        remaining_days -= DAYS_IN_YEAR[leap_year] as u64;
        year += 1;
    }

    // Determine the current month and day
    let leap_year = if (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0) { 1 } else { 0 };
    let mut month = 0;
    while remaining_days >= DAYS_IN_MONTH[leap_year][month] as u64 {
        remaining_days -= DAYS_IN_MONTH[leap_year][month] as u64;
        month += 1;
    }
    let day = remaining_days + 1; // Days are 1-based

    // Calculate the current hour, minute, and second
    let remaining_seconds = seconds % SECONDS_IN_DAY;
    let hour = (remaining_seconds / SECONDS_IN_HOUR) as u32;
    let minute = ((remaining_seconds % SECONDS_IN_HOUR) / SECONDS_IN_MINUTE) as u32;
    let second = (remaining_seconds % SECONDS_IN_MINUTE) as u32;

    (year, month as u32 + 1, day as u32, hour, minute, second)
}
  pub fn get_current_timestamp() -> String {
    let now = SystemTime::now();
    let since_epoch = now.duration_since(UNIX_EPOCH).expect("Time went backwards");
    let secs = since_epoch.as_secs();
    let (year, month, day, hour, minute, second) = Self::epoch_to_ymdhms(secs);

    
    format!("{:04}/{:02}/{:02} - {:02}:{:02}:{:02}",
    year, month, day, hour, minute, second)
  }
}



pub struct Logger<W: ?Sized + Write> {
  buffer: BufWriter<W>,
}

impl<W: Write> Logger<W> {
  pub fn new(writer: W) -> Logger<W> {
    Logger {
      buffer: BufWriter::new(writer)
    }
  }

  fn log(&mut self, content: String) {
    let _ = self.buffer.write(content.as_bytes());
    let _ = self.buffer.flush();
  } 

  pub fn msg(&mut self, content: String) {
    self.log(format!("[{}] - {}\n",SimpleTime::get_current_timestamp() ,content));
  }

}

#[cfg(test)]
use std::io::stdout;

#[test]
fn test_basic() {

    let mut logs = Logger::new(stdout()); 
    logs.msg("test".to_string());
}