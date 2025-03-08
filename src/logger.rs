use std::io::Write;
use std::sync::mpsc::{channel, Sender};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

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
            [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31], // Leap years
        ];

        // Calculate the number of days since the epoch
        let mut remaining_days = seconds / SECONDS_IN_DAY;

        // Determine the current year
        let mut year = 1970;
        loop {
            let leap_year = if (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0) {
                1
            } else {
                0
            };
            if remaining_days < DAYS_IN_YEAR[leap_year] as u64 {
                break;
            }
            remaining_days -= DAYS_IN_YEAR[leap_year] as u64;
            year += 1;
        }

        // Determine the current month and day
        let leap_year = if (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0) {
            1
        } else {
            0
        };
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

        format!(
            "{:04}/{:02}/{:02} - {:02}:{:02}:{:02}",
            year, month, day, hour, minute, second
        )
    }
}

pub struct Logger {
    tx: Sender<String>,
    _thread: JoinHandle<()>,
}

impl Logger {
    pub fn new<W: Sized + Write + Send + Sync + 'static>(mut writer: W) -> Logger {
        let (tx, rx) = channel::<String>();
        let thread = thread::spawn(move || {
            let mut last_flush = Instant::now();
            let mut buff = Vec::new();
            let mut max_size = 100;
            let timeout = Duration::from_secs(1);
            loop {
                let msg = rx.recv_timeout(timeout);
                match msg {
                    Ok(msg) => buff.push(msg),
                    Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
                    Err(_) => break,
                }

                if last_flush.elapsed() >= timeout || buff.len() >= max_size {
                    if !buff.is_empty() {
                        if buff.len() >= max_size {
                            max_size = (max_size * 10).min(1_000_000);
                        } else {
                            max_size = (max_size / 10).max(100);
                        }
                        let wr = writer.write_all(buff.join("").as_bytes());
                        if wr.is_err() {
                            println!("{:?}", wr);
                        }
                        let _ = writer.flush();

                        buff.clear();
                    }
                    last_flush = Instant::now();
                }
            }
        });
        Logger {
            tx,
            _thread: thread,
        }
    }

    pub fn msg(&self, content: String) {
        let content = format!("[{}] - {}\n", SimpleTime::get_current_timestamp(), content);
        let _ = self.tx.send(content);
    }
}

// #[cfg(test)]
// use std::io::stdout;

// #[test]
// fn test_basic() {
//     let mut logs = Logger::new(stdout());
//     logs.msg("test".to_string());
// }
