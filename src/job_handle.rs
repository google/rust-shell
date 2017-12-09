use ::libc;
use ::libc::c_int;
use result::ShellError;
use result::ShellResult;
use ::signal_handler::SignalHandler;
use ::std::mem;

struct JobHandleData {
    pid: c_int,
    /// out for capture
    out: Option<c_int>
}

/// Job which is a process leader.
pub struct JobHandle(Option<JobHandleData>);

impl JobHandle {
    pub fn new(pid: c_int, out: Option<c_int>) -> JobHandle {
        JobHandle(Some(JobHandleData {
            pid: pid,
            out: out
        }))
    }

    /// Sends a SIGTERM to a process group, then wait a process leader.
    pub fn terminate(self) -> ShellResult {
        SignalHandler::signal(self.0.as_ref().unwrap().pid, libc::SIGTERM)?;
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
        let data = mem::replace(&mut self.0, None).unwrap();
        let pid = data.pid;
        SignalHandler::wait(pid)
    }
}

impl Drop for JobHandle {
    fn drop(&mut self) {
        if self.0.is_none() {
            return;
        }
        self.wait_mut().unwrap();
    }
}
