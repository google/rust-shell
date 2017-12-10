use libc::c_int;
use libc;
use pipe_capture::PipeCapture;
use process_manager::ChildProcess;
use process_manager::PROCESS_MANAGER;
use result::ShellError;
use result::ShellResult;
use std::mem;
use std::sync::Arc;
use std::sync::RwLock;

struct JobHandleData {
    process: Arc<RwLock<ChildProcess>>,
    pid: c_int,
    /// out for capture
    out: Option<PipeCapture>
}

/// Job which is a process leader.
pub struct JobHandle(Option<JobHandleData>);

impl JobHandle {
    pub fn new(process: Arc<RwLock<ChildProcess>>,
               pid: c_int, out: Option<PipeCapture>) -> JobHandle {
        JobHandle(Some(JobHandleData {
            process: process,
            pid: pid,
            out: out
        }))
    }

    pub fn signal(&self, signal: c_int) -> ShellResult {
        let process = &self.0.as_ref().unwrap().process;
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
        let data = mem::replace(&mut self.0, None).unwrap();
        {
            let data = data.process.read().unwrap();
            data.wait_null()?;
        }
        {
            let mut data = data.process.write().unwrap();
            data.wait_mut()?;
        }
        {
            let mut process_manager = PROCESS_MANAGER.lock().unwrap();
            process_manager.remove_job(&data.process);
        }
        Ok(())
    }
}

impl Drop for JobHandle {
    fn drop(&mut self) {
        if self.0.is_none() {
            return;
        }
        self.wait_mut();
    }
}
