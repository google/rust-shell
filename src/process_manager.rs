use Executable;
use job_handle::JobHandle;
use job_spec::Redirect;
use libc::c_int;
use libc;
use pipe_capture::PipeCapture;
use result::ShellError;
use result::ShellResult;
use result::ShellResultExt;
use result::check_errno;
use std::cell::RefCell;
use std::mem;
use std::panic;
use std::process;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::RwLock;
use std::thread::ThreadId;
use std::thread;
use local_shell::LocalShell;

#[derive(Debug)]
struct ChildProcessData {
    pid: c_int,
    has_group: bool
}

#[derive(Debug)]
pub struct ChildProcess(Option<ChildProcessData>);

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

    pub fn signal(&self, sig: c_int) -> ShellResult {
        if self.0.is_none() {
            return Err(ShellError::NoSuchProcess);
        }
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

    pub fn wait_null(&self) -> ShellResult {
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

    pub fn wait_mut(&mut self) -> ShellResult {
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

struct ProcessEntry {
    thread_id: ThreadId,
    process: Arc<RwLock<ChildProcess>>
}

/// Managing global child process state.
pub struct ProcessManager {
    children: Vec<ProcessEntry>
}

impl ProcessManager {
    extern fn handle_signal(signal: c_int) {
        ::std::thread::spawn(move || {
            let mut lock = PROCESS_MANAGER.lock().unwrap();
            for entry in lock.children.drain(..) {
                entry.process.read().unwrap().signal(signal).print_error();
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
            children: Vec::new()
        }
    }

    pub fn add_local_shell(&mut self, shell: &Arc<RefCell<LocalShell>>) {
    }

    pub fn remove_local_shell(&mut self, shell: &Arc<RefCell<LocalShell>>) {
    }

    pub fn remove_job(&mut self, job: &Arc<RwLock<ChildProcess>>) {
        self.children.retain(|entry| !Arc::ptr_eq(&entry.process, job));
    }

    pub fn signal_thread_jobs(&mut self, id: &ThreadId, signal: c_int) {
        for entry in &self.children {
            if entry.thread_id != *id {
                continue;
            }
            entry.process.read().unwrap().signal(signal).print_error();
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
                let mut lock = PROCESS_MANAGER.lock().unwrap();
                let pid = check_errno("fork", libc::fork())?;
                if pid != 0 {
                    let mut child = ChildProcess::new(pid);
                    if process_group {
                        check_errno("setpgid", libc::setpgid(pid, 0)).unwrap();
                        child.set_has_group(true);
                    }
                    let arc = Arc::new(RwLock::new(child));
                    lock.children.push(ProcessEntry {
                        thread_id: thread::current().id(),
                        process: arc.clone()
                    });
                    if let Some(capture) = stdout.as_mut() {
                        capture.start_reading();
                    }
                    return Ok(JobHandle::new(arc, pid, stdout));
                } else {
                    lock.children.clear();
                    match stdout {
                        Some(stdout) => stdout.start_writing()?,
                        None => ()
                    }
                }
                pid
            };
            let mutex = Mutex::new(executor);
            let result = panic::catch_unwind(move || {
                if process_group {
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
}

lazy_static! {
    pub static ref PROCESS_MANAGER: Mutex<ProcessManager> =
        Mutex::new(ProcessManager::new());
}
