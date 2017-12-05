use ::libc::c_int;
use ::result::ShellResultExt;
use ::std::sync::Arc;
use ::std::sync::Mutex;

pub struct SignalHandler {
    pids: Vec<c_int>
}

extern fn handle_signal(signal: c_int) {
    ::std::thread::spawn(move || {
        match SIGNAL_HANDLER.lock() {
            Ok(signal_handler) => signal_handler.kill_all(signal),
            Err(_) => ::std::process::exit(128 + signal),
        }
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
        SignalHandler {
            pids: Vec::new()
        }
    }

    pub fn kill_all(&self, signal: c_int) -> ! {
        unsafe {
            for pid in &self.pids {
                ::result::check_errno(
                    "kill", ::libc::kill(*pid, signal)).print_error();
            }
            for pid in &self.pids {
                let mut status: i32 = 0;
                ::result::check_errno(
                    "waitpid", ::libc::waitpid(
                        pid.abs(), &mut status, 0)).print_error();
            }
        }
        ::std::process::exit(128 + signal)
    }

    pub fn add_pid(&mut self, pid: c_int) {
        self.pids.push(pid);
    }
}

lazy_static! {
    pub static ref SIGNAL_HANDLER: Arc<Mutex<SignalHandler>> =
        Arc::new(Mutex::new(SignalHandler::new()));
}
