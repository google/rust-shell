use ::libc;
use ::libc::c_int;
use result::ShellError;
use result::ShellResult;
use ::process_manager::ProcessManager;
use ::std::mem;
use pipe_capture::PipeCapture;

struct JobHandleData {
    pid: c_int,
    /// out for capture
    out: Option<PipeCapture>
}

/// Job which is a process leader.
pub struct JobHandle(Option<JobHandleData>);

impl JobHandle {
    pub fn new(pid: c_int, out: Option<PipeCapture>) -> JobHandle {
        JobHandle(Some(JobHandleData {
            pid: pid,
            out: out
        }))
    }

    /// Sends a SIGTERM to a process group, then wait a process leader.
    pub fn terminate(self) -> ShellResult {
        ProcessManager::signal(self.0.as_ref().unwrap().pid, libc::SIGTERM)?;
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
        ProcessManager::wait(pid)
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
