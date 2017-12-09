use ::libc;
use ::libc::c_int;
use result::ShellError;
use result::ShellResult;
use ::signal_handler::SignalHandler;
use ::std::mem;

/// Job which is a process leader.
pub struct JobHandle(Option<c_int>);

impl JobHandle {
    pub fn new(pid: c_int) -> JobHandle {
        JobHandle(Some(pid))
    }

    /// Sends a SIGTERM to a process group, then wait a process leader.
    pub fn terminate(self) -> ShellResult {
        SignalHandler::signal(self.0.unwrap(), libc::SIGTERM)?;
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
        let pid = mem::replace(&mut self.0, None).unwrap();
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
