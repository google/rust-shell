use ::libc;
use ::libc::c_int;
use ::Executable;
use ::result::check_errno;
use ::result::ShellError;
use ::result::ShellResult;
use ::result::ShellResultExt;
use ::std::sync::Arc;
use ::std::sync::Mutex;
use ::std::mem;
use ::std::panic;
use ::std::process;
use ::job_handle::JobHandle;
use ::std::collections::HashMap;

struct ChildProcessData {
    pid: c_int,
    has_group: bool
}

struct ChildProcess(Option<ChildProcessData>);

impl ChildProcess {
    fn new(pid: c_int) -> ChildProcess {
        ChildProcess(Some(ChildProcessData {
            pid: pid,
            has_group: false
        }))
    }

    fn set_has_group(&mut self, value: bool) {
        self.0.as_mut().unwrap().has_group = value;
    }

    fn forget(mut self) {
        self.0 = None;
    }

    fn signal(&self, sig: c_int) -> ShellResult {
        let data = self.0.as_ref().unwrap();
        let kill_pid = if data.has_group {
            -data.pid
        } else {
            data.pid
        };
        unsafe {
            check_errno("kill", libc::kill(kill_pid, sig))?;
        }
        Ok(())
    }

    fn wait(&mut self) -> ShellResult {
        let data = mem::replace(&mut self.0, None).unwrap();
        let pid = data.pid;
        loop {
            unsafe {
                let mut status: c_int = 0;
                check_errno("waitpid", libc::waitpid(
                        pid, &mut status as *mut i32, 0))?;
                if libc::WIFEXITED(status) {
                    let code = libc::WEXITSTATUS(status);
                    if code == 0 {
                        return Ok(());
                    } else {
                        return Err(ShellError::Code(code as u8));
                    }
                } else if libc::WIFSIGNALED(status) {
                    let signal = libc::WTERMSIG(status);
                    return Err(ShellError::Signaled(signal));
                }
            }
        }
    }
}

impl Drop for ChildProcess {
    fn drop(&mut self) {
        if self.0.is_none() {
            return;
        }
        self.wait().print_error();
    }
}

/// Managing global child process state.
pub struct SignalHandler {
    children: HashMap<c_int, ChildProcess>
}

impl SignalHandler {
    extern fn handle_signal(signal: c_int) {
        ::std::thread::spawn(move || {
            let mut lock = SIGNAL_HANDLER.lock();
            if let Ok(ref mut signal_handler) = lock {
                for child in signal_handler.children.values() {
                    child.signal(signal).print_error();
                }
            }
            ::std::process::exit(128 + signal);
        });
    }

    fn new() -> SignalHandler {
        let result = unsafe {
            libc::signal(libc::SIGINT, SignalHandler::handle_signal as usize)
        };
        if result == ::libc::SIG_ERR {
            panic!("signal failed");
        }
        let result = unsafe {
            libc::signal(libc::SIGTERM, SignalHandler::handle_signal as usize)
        };
        if result == ::libc::SIG_ERR {
            panic!("signal failed");
        }
        SignalHandler {
            children: HashMap::new()
        }
    }

    /// Fork new process
    pub fn fork(executor: Box<Executable>, process_group: bool) 
            -> Result<JobHandle, ShellError> {
        unsafe {
            let pid = {
                let mut lock = SIGNAL_HANDLER.lock().unwrap();
                let pid = check_errno("fork", libc::fork())?;
                if pid != 0 {
                    let mut child = ChildProcess::new(pid);
                    if process_group {
                        check_errno("setpgid", libc::setpgid(pid, 0)).unwrap();
                        child.set_has_group(true);
                    }
                    lock.children.insert(pid, child);
                    return Ok(JobHandle::new(pid));
                } else {
                    for child in lock.children.drain() {
                        child.1.forget();
                    }
                }
                pid
            };

            let mutex = Mutex::new(executor);
            panic::catch_unwind(move || {
                if process_group {
                    check_errno("setpgid", libc::setpgid(pid, 0)).unwrap();
                }
                mutex.lock().unwrap().exec();
            }).is_ok();
            // The control reaches here only when the process was paniced.
            process::exit(101);
        }
    }

    pub fn signal(pid: c_int, signal: c_int) -> ShellResult {
        let signal_handler = SIGNAL_HANDLER.lock().unwrap();
        let child = signal_handler.children.get(&pid).unwrap();
        child.signal(signal)
    }

    pub fn wait(pid: c_int) -> ShellResult {
        let mut signal_handler = SIGNAL_HANDLER.lock().unwrap();
        let mut child = signal_handler.children.remove(&pid).unwrap();
        child.wait()
    }
}

lazy_static! {
    static ref SIGNAL_HANDLER: Arc<Mutex<SignalHandler>> =
        Arc::new(Mutex::new(SignalHandler::new()));
}
