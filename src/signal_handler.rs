use ::libc::c_int;
use ::result::ShellResultExt;
use ::std::sync::Arc;
use ::std::sync::Mutex;

pub struct SignalHandler {
    pids: Vec<c_int>,
    
}

extern fn handle_signal(signal: c_int) {
    ::std::thread::spawn(move || {
        let lock = SIGNAL_HANDLER.lock();
        if let Ok(mut signal_handler) = lock {
            signal_handler.kill_all(signal);
        }
        ::std::process::exit(128 + signal);
    });
}

impl SignalHandler {
    fn new() -> SignalHandler {
        let result = unsafe {
            ::libc::signal(::libc::SIGINT, handle_signal as usize)
        };
        if result == ::libc::SIG_ERR {
            panic!("signal failed");
        }
        let result = unsafe {
            ::libc::signal(::libc::SIGTERM, handle_signal as usize)
        };
        if result == ::libc::SIG_ERR {
            panic!("signal failed");
        }
        SignalHandler {
            pids: Vec::new()
        }
    }

    pub fn kill_all(&mut self, signal: c_int) {
        unsafe {
            for pid in &self.pids {
                ::result::check_errno(
                    "kill", ::libc::kill(*pid, signal)).print_error();
            }
        }
        self.clear();
    }

    pub fn add_pid(&mut self, pid: c_int) {
        self.pids.push(pid);
    }

    pub fn clear(&mut self) {
        self.pids.clear();
    }
}

lazy_static! {
    pub static ref SIGNAL_HANDLER: Arc<Mutex<SignalHandler>> =
        Arc::new(Mutex::new(SignalHandler::new()));
}
