use core::panic;
use std::mem::zeroed;
use std::net::Ipv4Addr;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::Duration;
use std::{ptr, thread};

use libc::{
    AF_INET, SA_RESTART, SOCK_STREAM, close, connect, htons, in_addr, sigaction, sighandler_t,
    sockaddr, sockaddr_in, socket,
};

use crate::hteapot::Hteapot;
use crate::logger::Logger;

pub fn setup_graceful_shutdown(server: &mut Hteapot, logger: Logger) -> Arc<AtomicBool> {
    let existing_signal = server.get_shutdown_signal();
    if existing_signal.is_some() {
        return existing_signal.unwrap();
    }
    let running = Arc::new(AtomicBool::new(true));
    let shutdown_logger = logger.with_component("shutdown");

    //This is a simplification an a ad-hoc solution
    #[cfg(unix)]
    {
        fn to_sockaddr_in(addr: (String, i16)) -> sockaddr_in {
            let ip: Ipv4Addr = addr.0.parse().expect("IP inválida");
            let port = addr.1 as u16;

            sockaddr_in {
                sin_family: libc::AF_INET as u8,
                #[cfg(any(
                    target_os = "macos",
                    target_os = "ios",
                    target_os = "freebsd",
                    target_os = "netbsd",
                    target_os = "openbsd"
                ))]
                sin_len: std::mem::size_of::<sockaddr_in>() as u8,
                sin_port: htons(port),
                sin_addr: in_addr {
                    s_addr: u32::from_ne_bytes(ip.octets()),
                },
                sin_zero: [0; 8],
            }
        }
        unsafe {
            // safety guard to avoid editions of RUNNING_PTR
            // this will change whit multi server support
            if !RUNNING_PTR.is_null() {
                panic!("Tried to setup shutdown for two different server instances");
            }
        }
        static mut RUNNING_PTR: *const AtomicBool = ptr::null();
        static COUNTER_PTR: AtomicUsize = AtomicUsize::new(0);

        extern "C" fn handle_sigint(_: i32) {
            unsafe {
                if COUNTER_PTR.load(Ordering::SeqCst) < 9 {
                    COUNTER_PTR.fetch_add(1, Ordering::SeqCst);
                    if COUNTER_PTR.load(Ordering::SeqCst) == 9 {
                        println!("\rPress ctrl+c one more time to force quit");
                    }
                } else {
                    println!("\rForcing exit, now!");
                    std::process::exit(0);
                }

                if !RUNNING_PTR.is_null() {
                    (*RUNNING_PTR).store(false, Ordering::SeqCst);
                    let fd = socket(AF_INET, SOCK_STREAM, 0);
                    let addr = to_sockaddr_in(("0.0.0.0".to_string(), 8081));
                    let _ = connect(
                        fd,
                        &addr as *const sockaddr_in as *const sockaddr,
                        size_of::<sockaddr_in>() as u32,
                    );

                    // cerramos el socket aunque haya fallado
                    close(fd);
                }
            }
        }

        unsafe {
            ///////////////////////////////////////////////////////////////////////////
            // Create a raw pointer and increase the reference counter to avoid
            // UB and early deallocation. IMPORTANT: remember to decrement if in the
            // future there is a function to disable this ctrl+c logic
            ///////////////////////////////////////////////////////////////////////////

            RUNNING_PTR = Arc::as_ptr(&running);
            Arc::increment_strong_count(RUNNING_PTR);

            let mut action: sigaction = zeroed();
            action.sa_flags = SA_RESTART;
            action.sa_sigaction = handle_sigint as sighandler_t;
            sigaction(libc::SIGINT, &action, std::ptr::null_mut());
        }
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
