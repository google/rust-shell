use ::libc;
use result::ShellError;
use result::ShellResult;
use result::check_errno;

/// Job which is a process leader.
pub struct JobHandle { pub pid: i32, pub setpgid: bool }

impl JobHandle {
    /// Sends a SIGTERM to a process group, then wait a process leader.
    pub fn terminate(self) -> ShellResult {
        assert_ne!(self.pid, 0);
        unsafe {
            let pid = if self.setpgid {
                -self.pid
            } else {
                self.pid
            };
            check_errno("kill", libc::kill(pid, libc::SIGTERM))?;
            match self.wait() {
                Ok(()) | Err(ShellError::Code(_)) 
                    | Err(ShellError::Signaled(_)) => Ok(()),
                err => err
            }
        }
    }

    /// Wait for termination of the process.
    pub fn wait(mut self) -> ShellResult {
        self.wait_mut()
    }

    fn wait_mut(&mut self) -> ShellResult {
        if self.pid == 0 {
            return Ok(());
        }
        let pid = self.pid;
        self.pid = 0;
        loop {
            unsafe {
                let mut status: libc::c_int = 0;
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

impl Drop for JobHandle {
    fn drop(&mut self) {
        self.wait_mut().unwrap();
    }
}
