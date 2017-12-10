use libc::c_int;
use libc;
use result::ShellError;
use result::ShellResult;
use std::mem;
use std::sync::Arc;
use std::sync::RwLock;
use local_shell::LOCAL_SHELL;
use result::check_errno;
use std::process::Child;
use std::process::Command;
use std::process::ExitStatus;

#[derive(Debug)]
struct ChildProcessData {
    child: Child,
    has_group: bool
}

#[derive(Debug)]
pub struct ChildProcess(Option<ChildProcessData>);

impl ChildProcess {
    fn new(child: Child, has_group: bool) -> ChildProcess {
        ChildProcess(Some(ChildProcessData {
            child: child,
            has_group: has_group
        }))
    }

    pub fn signal(&self, sig: c_int) -> ShellResult {
        let data = match &self.0 {
            &Some(ref data) => data,
            &None => return Err(ShellError::NoSuchProcess)
        };
        let kill_pid = if data.has_group {
            -(data.child.id() as i32)
        } else {
            data.child.id() as i32
        };
        unsafe {
            check_errno("kill", libc::kill(kill_pid, sig))?;
        }
        Ok(())
    }

    pub fn wait_null(&self) -> ShellResult {
        let data = match &self.0 {
            &Some(ref data) => data,
            &None => return Err(ShellError::NoSuchProcess)
        };
        unsafe {
            let mut info = mem::uninitialized::<libc::siginfo_t>();
            check_errno("waitid",
                        libc::waitid(
                            libc::P_PID,
                            data.child.id() as u32,
                            &mut info as *mut libc::siginfo_t,
                            libc::WEXITED | libc::WNOWAIT))?;
        }
        Ok(())
    }

    pub fn wait_mut(&mut self) -> Result<ExitStatus, ShellError> {
        let mut data = match mem::replace(&mut self.0, None) {
            Some(data) => data,
            None => return Err(ShellError::NoSuchProcess)
        };
        Ok(data.child.wait()?)
    }
}

/// Job which is a process leader.
pub struct JobHandle(Arc<RwLock<ChildProcess>>);

impl JobHandle {
    pub fn new(mut command: Command, has_group: bool) 
            -> Result<JobHandle, ShellError> {
        LOCAL_SHELL.with(|shell| {
            let mut lock = shell.lock().unwrap();
            let child = command.spawn()?;
            let process = Arc::new(RwLock::new(
                    ChildProcess::new(child, has_group)));
            lock.add_process(&process);
            Ok(JobHandle(process))
        })
    }

    pub fn signal(&self, signal: c_int) -> ShellResult {
        let process = &self.0;
        let process = process.read().unwrap();
        process.signal(signal)
    }

    /// Sends a SIGTERM to a process group, then wait a process leader.
    pub fn terminate(self) -> ShellResult {
        self.signal(libc::SIGTERM)?;
        match self.wait() {
            Ok(()) | Err(ShellError::Code(_))
                | Err(ShellError::Signaled(_)) => Ok(()),
            err => err
        }
    }

    /// Wait for termination of the process.
    pub fn wait(mut self) -> ShellResult {
        self.wait_mut()
    }

    fn wait_mut(&mut self) -> ShellResult {
        {
            let data = self.0.read().unwrap();
            data.wait_null()?;
        }
        {
            let mut data = self.0.write().unwrap();
            data.wait_mut()?;
        }
        LOCAL_SHELL.with(|shell| {
            let mut lock = shell.lock().unwrap();
            lock.remove_process(&self.0)
        });
        Ok(())
    }
}

impl Drop for JobHandle {
    fn drop(&mut self) {
        self.wait_mut();
    }
}
