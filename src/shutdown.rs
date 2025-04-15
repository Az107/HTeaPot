use std::net::TcpStream;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;

use crate::hteapot::Hteapot;
use crate::logger::Logger;

pub fn setup_graceful_shutdown(server: &mut Hteapot, logger: Logger) -> Arc<AtomicBool> {
    let existing_signal = server.get_shutdown_signal();
    if existing_signal.is_some() {
        return existing_signal.unwrap();
    }
    let running = Arc::new(AtomicBool::new(true));
    let shutdown_logger = logger.with_component("shutdown");

    #[cfg(unix)]
    {
        let mut ush = unix_signhandler::UnixSignHandler::new();
        let running_clone = running.clone();
        let addr = server.get_addr();
        ush.set_handler(move || {
            running_clone.store(false, Ordering::SeqCst);
            let _ = TcpStream::connect(format!("{}:{}", addr.0, addr.1));
        });
    }

    #[cfg(windows)]
    {
        let r_win = running.clone();
        let win_logger = shutdown_logger.clone();

        if !win_console::set_handler(r_win, win_logger.clone()) {
            win_logger.error("Failed to set Windows Ctrl+C handler".to_string());
        } else {
            win_logger.info("Windows Ctrl+C handler registered".to_string());
        }
    }

    // Add shutdown hook for cleanup
    let shutdown_logger_clone = shutdown_logger.clone();
    server.add_shutdown_hook(move || {
        shutdown_logger_clone.info("Waiting for ongoing requests to complete...".to_string());
        thread::sleep(Duration::from_secs(3));
        shutdown_logger_clone.info("Exiting".to_string());
    });

    server.set_shutdown_signal(running.clone());
    // Return the running flag so the main thread can also check it
    running
}

#[cfg(windows)]
pub mod win_console {
    use std::sync::Arc;
    use std::sync::Mutex;
    use std::sync::atomic::{AtomicBool, Ordering};

    // Define the external Windows API function in an unsafe extern block
    unsafe extern "system" {
        pub fn SetConsoleCtrlHandler(
            handler: Option<unsafe extern "system" fn(ctrl_type: u32) -> i32>,
            add: i32,
        ) -> i32;
    }

    pub const CTRL_C_EVENT: u32 = 0;

    struct StaticData {
        running: Option<Arc<AtomicBool>>,
        logger: Option<crate::logger::Logger>,
    }

    // We need to ensure thread safety, so use a Mutex for it
    static HANDLER_DATA: Mutex<StaticData> = Mutex::new(StaticData {
        running: None,
        logger: None,
    });

    pub fn set_handler(running: Arc<AtomicBool>, logger: crate::logger::Logger) -> bool {
        // Store references in the mutex-protected static
        let mut data = HANDLER_DATA.lock().unwrap();
        data.running = Some(running);
        data.logger = Some(logger);

        unsafe extern "system" fn handler_func(ctrl_type: u32) -> i32 {
            if ctrl_type == CTRL_C_EVENT {
                if let Ok(data) = HANDLER_DATA.lock() {
                    if let (Some(r), Some(l)) = (&data.running, &data.logger) {
                        l.info("initiating graceful shutdown...".to_string());
                        r.store(false, Ordering::SeqCst);
                        std::process::exit(0);
                    }
                }
            }
            0
        }

        unsafe { SetConsoleCtrlHandler(Some(handler_func), 1) != 0 }
    }
}

//Brought to you by: Overengeneering DIY™️
#[cfg(unix)]
mod unix_signhandler {
    use libc::{POLLIN, SA_RESTART, poll, pollfd, sigaction, sighandler_t};
    use std::io;
    use std::sync::{Arc, RwLock};
    use std::{mem::zeroed, os::fd::RawFd, thread};

    static mut PIPE_FD_READ: RawFd = -1;
    static mut PIPE_FD_WRITE: RawFd = -1;
    extern "C" fn handler(_: i32) {
        let buf = [1u8];
        unsafe {
            if PIPE_FD_WRITE != -1 {
                let _ = libc::write(PIPE_FD_WRITE, buf.as_ptr() as *const _, 1);
            }
        }
    }

    fn wait_for_readable(fd: RawFd) -> io::Result<()> {
        let mut fds = [pollfd {
            fd,
            events: POLLIN,
            revents: 0,
        }];
        let ret = unsafe { poll(fds.as_mut_ptr(), 1, -1) }; // -1 = undefined timeout

        if ret < 0 {
            return Err(io::Error::last_os_error());
        }

        if fds[0].revents & POLLIN != 0 {
            Ok(())
        } else {
            Err(io::Error::new(
                io::ErrorKind::Other,
                "unexpected poll result",
            ))
        }
    }

    pub struct UnixSignHandler {
        actions: Arc<RwLock<Vec<Box<dyn Fn() + Send + Sync + 'static>>>>,
    }

    impl UnixSignHandler {
        pub fn new() -> Self {
            let ush = UnixSignHandler {
                actions: Arc::new(RwLock::new(Vec::new())),
            };
            unsafe {
                let mut fds = [0; 2];
                if libc::pipe(fds.as_mut_ptr()) == -1 {
                    panic!("failed to create pipe");
                }
                PIPE_FD_READ = fds[0];
                PIPE_FD_WRITE = fds[1];
            }
            unsafe {
                let mut action: sigaction = zeroed();
                action.sa_flags = SA_RESTART;
                action.sa_sigaction = handler as sighandler_t;
                sigaction(libc::SIGINT, &action, std::ptr::null_mut());
            }

            let actions_clone = ush.actions.clone();
            thread::spawn(move || {
                unsafe {
                    let _ = wait_for_readable(PIPE_FD_READ);
                }
                for action in actions_clone.read().unwrap().iter() {
                    action();
                }
            });
            return ush;
        }
        pub fn set_handler(&mut self, action: impl Fn() + Send + Sync + 'static) {
            self.actions.write().unwrap().push(Box::new(action));
        }
    }
}
