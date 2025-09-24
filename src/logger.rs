use std::fmt;
use std::io::Write;
use std::sync::Arc;
use std::sync::mpsc::{Sender, channel};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// Differnt log levels
#[derive(Debug, Clone, PartialEq, Eq, Ord, PartialOrd, Copy)]
#[allow(dead_code)]
pub enum LogLevel {
    DEBUG,
    INFO,
    WARN,
    ERROR,
    FATAL,
    TRACE,
}

impl fmt::Display for LogLevel {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            LogLevel::DEBUG => write!(f, "DEBUG"),
            LogLevel::INFO => write!(f, "INFO"),
            LogLevel::WARN => write!(f, "WARN"),
            LogLevel::ERROR => write!(f, "ERROR"),
            LogLevel::FATAL => write!(f, "FATAL"),
            LogLevel::TRACE => write!(f, "TRACE"),
        }
    }
}

/// A helper struct for generating formatted timestamps in `YYYY-MM-DD HH:MM:SS.sss` format.
struct SimpleTime;
impl SimpleTime {
    fn epoch_to_ymdhms(seconds: u64, nanos: u32) -> (i32, u32, u32, u32, u32, u32, u32) {
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

        // calculate millisecs from nanosecs
        let millis = nanos / 1_000_000;

        (
            year,
            month as u32 + 1,
            day as u32,
            hour,
            minute,
            second,
            millis,
        )
    }

    /// Returns a formatted timestamp string for the current system time.
    pub fn get_current_timestamp() -> String {
        let now = SystemTime::now();
        let since_epoch = now.duration_since(UNIX_EPOCH).expect("Time went backwards");
        let secs = since_epoch.as_secs();
        let nanos = since_epoch.subsec_nanos();
        let (year, month, day, hour, minute, second, millis) = Self::epoch_to_ymdhms(secs, nanos);

        format!(
            "{:04}-{:02}-{:02} {:02}:{:02}:{:02}.{:03}",
            year, month, day, hour, minute, second, millis
        )
    }
}

/// Internal message structure containing log metadata.
struct LogMessage {
    timestamp: String,
    level: LogLevel,
    component: String,
    content: String,
}

/// Core logger implementation running in a background thread.
struct LoggerCore {
    tx: Sender<LogMessage>,
    _thread: JoinHandle<()>,
}

/// A non-blocking, multi-threaded logger with support for log levels and components.
///
/// Logs are sent via a background thread to avoid blocking the main thread.
/// The logger buffers messages and flushes them periodically or based on buffer size.
pub struct Logger {
    core: Arc<LoggerCore>,
    component: Arc<String>,
    min_level: LogLevel,
}

impl Logger {
    /// Creates a new logger with the given writer, minimum log level, and component name.
    pub fn new<W: Sized + Write + Send + Sync + 'static>(
        mut writer: W,
        min_level: LogLevel,
        component: &str,
    ) -> Logger {
        let (tx, rx) = channel::<LogMessage>();
        let thread = thread::spawn(move || {
            let mut last_flush = Instant::now();
            let mut buff = Vec::new();
            let mut max_size = 100;
            let timeout = Duration::from_secs(1);

            loop {
                let msg = rx.recv_timeout(timeout);
                match msg {
                    Ok(msg) => {
                        let formatted = format!(
                            "\r{} [{}] [{}] {}\n",
                            msg.timestamp, msg.level, msg.component, msg.content
                        );
                        buff.push(formatted);
                    }
                    Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
                    Err(_) => break,
                }

                // Flush if timeout or buffer threshold reached
                if last_flush.elapsed() >= timeout || buff.len() >= max_size {
                    if !buff.is_empty() {
                        if buff.len() >= max_size {
                            max_size = (max_size * 10).min(1_000_000);
                        } else {
                            max_size = (max_size / 10).max(100);
                        }
                        let wr = writer.write_all(buff.join("").as_bytes());
                        if wr.is_err() {
                            println!("Failed to write to log: {:?}", wr);
                        }
                        let _ = writer.flush();

                        buff.clear();
                    }
                    last_flush = Instant::now();
                }
            }
        });

        Logger {
            core: Arc::new(LoggerCore {
                tx,
                _thread: thread,
            }),
            component: Arc::new(component.to_string()),
            min_level,
        }
    }

    /// Creates a new logger instance with a different component name, but sharing same output.
    pub fn with_component(&self, component: &str) -> Logger {
        Logger {
            core: Arc::clone(&self.core),
            component: Arc::new(component.to_string()),
            min_level: self.min_level,
        }
    }

    /// Sends a log message with the given level and content.
    pub fn log(&self, level: LogLevel, content: String) {
        if level < self.min_level {
            return;
        }

        let log_msg = LogMessage {
            timestamp: SimpleTime::get_current_timestamp(),
            level,
            component: (*self.component).clone(),
            content,
        };

        // Send the log message to the channel
        let _ = self.core.tx.send(log_msg);
    }

    /// Logs a DEBUG-level message.
    pub fn debug(&self, content: String) {
        self.log(LogLevel::DEBUG, content);
    }

    /// Log a message with INFO level
    pub fn info(&self, content: String) {
        self.log(LogLevel::INFO, content);
    }

    /// Log a message with WARN level
    pub fn warn(&self, content: String) {
        self.log(LogLevel::WARN, content);
    }

    /// Log a message with ERROR level
    pub fn error(&self, content: String) {
        self.log(LogLevel::ERROR, content);
    }

    /// Log a message with FATAL level
    #[allow(dead_code)]
    pub fn fatal(&self, content: String) {
        self.log(LogLevel::FATAL, content);
    }
    /// Log a message with TRACE level
    #[allow(dead_code)]
    pub fn trace(&self, content: String) {
        self.log(LogLevel::TRACE, content);
    }
}

impl Clone for Logger {
    fn clone(&self) -> Self {
        Logger {
            core: Arc::clone(&self.core),
            component: Arc::clone(&self.component),
            min_level: self.min_level,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::stdout;

    #[test]
    fn test_basic() {
        let logs = Logger::new(stdout(), LogLevel::DEBUG, "test");
        logs.info("test message".to_string());
        logs.debug("debug info".to_string());

        // Create a sub-logger with a different component
        let sub_logger = logs.with_component("sub-component");
        sub_logger.warn("warning from sub-component".to_string());
    }
}
