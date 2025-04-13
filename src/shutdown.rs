use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use crate::hteapot::Hteapot;
use crate::logger::Logger;

pub fn setup_graceful_shutdown(server: &mut Hteapot, logger: Logger) -> Arc<AtomicBool> {
    let running = Arc::new(AtomicBool::new(true));
    let r_server = running.clone();
    let shutdown_logger = logger.with_component("shutdown");
    
    #[cfg(unix)]
    {
        let r_unix = running.clone();
        let unix_logger = shutdown_logger.clone();
        unix_signal::register_signal_handler(r_unix, unix_logger);
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
    
    let r_enter = running.clone();
    let enter_logger = shutdown_logger.clone();
    
    thread::spawn(move || {
        println!(" Ctrl+C to shutdown the server gracefully...");
        let mut buffer = String::new();
        let _ = std::io::stdin().read_line(&mut buffer);
        enter_logger.info("Enter pressed, initiating graceful shutdown...".to_string());
        r_enter.store(false, Ordering::SeqCst);
    });
    
    // Set up server with shutdown signal
    server.set_shutdown_signal(r_server);
    
    // Add shutdown hook for cleanup
    let shutdown_logger_clone = shutdown_logger.clone();
    server.add_shutdown_hook(move || {
        shutdown_logger_clone.info("Waiting for ongoing requests to complete...".to_string());
        
        thread::sleep(Duration::from_secs(3));
        
        shutdown_logger_clone.info("Server shutdown complete.".to_string());
        
        std::process::exit(0);
    });
    
    // Return the running flag so the main thread can also check it
    running
}

#[cfg(unix)]
pub mod unix_signal {
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;
    use std::ptr;
    use std::mem;
    
    use libc::{c_int, c_void, sigaction, sighandler_t, sigset_t};
    use libc::{SA_RESTART, SIGINT, SIG_IGN};
    
    use crate::logger::Logger;
    
    // Global variables to store the signal handler state
    static mut RUNNING: Option<Arc<AtomicBool>> = None;
    static mut LOGGER: Option<Logger> = None;
    
    // Signal handler function
    extern "C" fn handle_signal(_: c_int) {
        unsafe {
            if let Some(running) = RUNNING.as_ref() {
                if let Some(logger) = LOGGER.as_ref() {
                    logger.info("SIGINT received, initiating graceful shutdown...".to_string());
                }
                running.store(false, Ordering::SeqCst);
            }
        }
    }
    
    pub fn register_signal_handler(running: Arc<AtomicBool>, logger: Logger) {
        unsafe {
            // Store our state in global variables
            RUNNING = Some(running);
            LOGGER = Some(logger.clone());
            
            // Set up the sigaction struct
            let mut sigact: sigaction = mem::zeroed();
            // Fix: Use the correct field name for the handler
            sigact.sa_sigaction = handle_signal as sighandler_t;
            sigact.sa_flags = SA_RESTART;
            
            // Empty the signal mask
            sigemptyset(&mut sigact.sa_mask);
            
            // Register our signal handler for SIGINT
            if sigaction(SIGINT, &sigact, ptr::null_mut()) < 0 {
                logger.error("Failed to set SIGINT handler".to_string());
            } else {
                logger.info("SIGINT handler registered".to_string());
            }
        }
    }
    
    // Helper function to create an empty signal set
    unsafe fn sigemptyset(set: *mut sigset_t) {
        // Fix: Add unsafe block around the unsafe operation
        unsafe {
            ptr::write_bytes(set, 0, 1);
        }
    }
}

#[cfg(windows)]
pub mod win_console {
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;
    use std::sync::Mutex;

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