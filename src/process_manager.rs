use ::Executable;
use ::job_handle::JobHandle;
use ::job_spec::Redirect;
use ::libc::c_int;
use ::libc;
use ::result::ShellError;
use ::result::ShellResult;
use ::result::ShellResultExt;
use ::result::check_errno;
use ::std::collections::HashMap;
use ::std::mem;
use ::std::panic;
use ::std::process;
use ::std::sync::Arc;
use ::std::sync::RwLock;
use ::std::sync::Mutex;
use ::pipe_capture::PipeCapture;

#[derive(Debug)]
struct ChildProcessData {
    pid: c_int,
    has_group: bool
}

#[derive(Debug)]
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

    fn wait_null(&self) -> ShellResult {
        unsafe {
            let mut info = mem::uninitialized::<libc::siginfo_t>();
            check_errno("waitid",
                        libc::waitid(
                            libc::P_PID,
                            self.0.as_ref().unwrap().pid as u32,
                            &mut info as *mut libc::siginfo_t,
                            libc::WEXITED | libc::WNOWAIT))?;
        }
        Ok(())
    }

    fn wait_mut(&mut self) -> ShellResult {
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
        self.wait_mut().print_error();
    }
}

/// Managing global child process state.
pub struct ProcessManager {
    children: HashMap<c_int, Arc<RwLock<ChildProcess>>>
}

impl ProcessManager {
    extern fn handle_signal(signal: c_int) {
        ::std::thread::spawn(move || {
            let mut lock = SIGNAL_HANDLER.lock();
            if let Ok(ref mut process_manager) = lock {
                for child in process_manager.children.values() {
                    child.read().unwrap().signal(signal).print_error();
                }
            }
            ::std::process::exit(128 + signal);
        });
    }

    fn new() -> ProcessManager {
        let result = unsafe {
            libc::signal(libc::SIGINT, ProcessManager::handle_signal as usize)
        };
        if result == ::libc::SIG_ERR {
            panic!("signal failed");
        }
        let result = unsafe {
            libc::signal(libc::SIGTERM, ProcessManager::handle_signal as usize)
        };
        if result == ::libc::SIG_ERR {
            panic!("signal failed");
        }
        ProcessManager {
            children: HashMap::new()
        }
    }

    /// Fork new process
    pub fn fork(executor: Box<Executable>, process_group: bool,
                stdin: Redirect, stdout: Redirect, stderr: Redirect)
            -> Result<JobHandle, ShellError> {
        unsafe {
            let mut stdout = match stdout {
                Redirect::Capture => Some(PipeCapture::new()?),
                _ => None
            };
            let pid = {
                let mut lock = SIGNAL_HANDLER.lock().unwrap();
                let pid = check_errno("fork", libc::fork())?;
                if pid != 0 {
                    let mut child = ChildProcess::new(pid);
                    if process_group {
                        check_errno("setpgid", libc::setpgid(pid, 0)).unwrap();
                        child.set_has_group(true);
                    }
                    lock.children.insert(pid, Arc::new(RwLock::new(child)));
                    if let Some(capture) = stdout.as_mut() {
                        capture.start_reading();
                    }
                    return Ok(JobHandle::new(pid, stdout));
                } else {
                    for child in lock.children.drain() {
                        // After fork ChildProcess should not track parent
                        // processes' ChildProcess. Thus we intentionally let
                        // them leaked.
                        mem::forget(child.1);
                    }
                    match stdout {
                        Some(stdout) => stdout.start_writing()?,
                        None => ()
                    }
                }
                pid
            };
            let mutex = Mutex::new(executor);
            let result = panic::catch_unwind(move || { if process_group {
                    check_errno("setpgid", libc::setpgid(pid, 0)).unwrap();
                }
                mutex.lock().unwrap().exec();
            });
            match result {
                Ok(_) => {
                    eprintln!("exec() does not exit the child process");
                    process::exit(1);
                }
                Err(error) => {
                    eprintln!("Child process paniced {:?}", error);
                    process::exit(101);
                }
            }
        }
    }

    pub fn signal(pid: c_int, signal: c_int) -> ShellResult {
        let process_manager = SIGNAL_HANDLER.lock().unwrap();
        let child = process_manager.children.get(&pid).unwrap();
        let child = child.read().unwrap();
        child.signal(signal)
    }

    pub fn wait(pid: c_int) -> ShellResult {
        let child = {
            let process_manager = SIGNAL_HANDLER.lock().unwrap();
            process_manager.children.get(&pid).unwrap().clone()
        };
        child.read().unwrap().wait_null()?;
        // Here child is zonbi state.
        let child = {
            let mut process_manager = SIGNAL_HANDLER.lock().unwrap();
            process_manager.children.remove(&pid).unwrap()
        };
        let mut child = child.write().unwrap();
        child.wait_mut()
    }
}

lazy_static! {
    static ref SIGNAL_HANDLER: Mutex<ProcessManager> =
        Mutex::new(ProcessManager::new());
}
