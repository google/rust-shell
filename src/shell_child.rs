use libc::c_int;
use libc;
use result::ShellError;
use result::ShellResult;
use std::mem;
use std::sync::Arc;
use std::sync::RwLock;
use result::check_errno;
use result::ShellResultExt;
use std::process::Child;
use std::process::Command;
use local_shell::current_shell;
use std::io::Read;

#[derive(Debug)]
pub struct ShellChildCore {
    command_line: String,
    pub child: Child,
    has_group: bool
}

impl ShellChildCore {
    fn new(command_line: String, child: Child,
           has_group: bool) -> ShellChildCore {
        ShellChildCore {
            command_line: command_line,
            child: child,
            has_group: has_group
        }
    }

    pub fn signal(&self, sig: c_int) -> ShellResult {
        let kill_pid = if self.has_group {
            -(self.child.id() as i32)
        } else {
            self.child.id() as i32
        };

        info!("Sending signal {} to {}", sig, self.child.id());
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
                            self.child.id() as u32,
                            &mut info as *mut libc::siginfo_t,
                            libc::WEXITED | libc::WNOWAIT))?;
        }
        Ok(())
    }

    pub fn wait(mut self) -> ShellResult {
        ShellResult::from_status(self.command_line, self.child.wait()?)
    }
}

pub type ShellChildArc = Arc<RwLock<Option<ShellChildCore>>>;

/// Job which is a process leader.
/// This wraps Arc<RwLock<ShellChildCore>> and provides helper functions.
pub struct ShellChild(pub ShellChildArc);

impl ShellChild {
    pub fn new(line: String, mut command: Command, has_group: bool)
            -> Result<ShellChild, ShellError> {
        let shell = current_shell();
        let mut lock = shell.lock().unwrap();
        if lock.signaled() {
            return Err(ShellError::from_signal(line, 101))
        }
        let child = command.spawn()?;
        let process = Arc::new(RwLock::new(
                Some(ShellChildCore::new(line, child, has_group))));
        lock.add_process(&process);
        Ok(ShellChild(process))
    }

    /// Sends a signal to the process.
    pub fn signal(&self, signal: c_int) -> ShellResult {
        let process = &self.0;
        let process = process.read().unwrap();
        process.as_ref().ok_or(ShellError::NoSuchProcess)?.signal(signal)
    }

    /// Waits for termination of the process.
    pub fn wait(self) -> ShellResult {
        {
            let data = self.0.read().unwrap();
            data.as_ref().ok_or(ShellError::NoSuchProcess)?.wait_null()?;
        }
        {
            let shell = current_shell();
            let mut lock = shell.lock().unwrap();
            lock.remove_process(&self.0);
        }
        let mut data = self.0.write().unwrap();
        data.take().ok_or(ShellError::NoSuchProcess)?.wait()
    }

    /// Obtains stdout as utf8 string.
    /// Returns Err if it returns non-zero exit code.
    pub fn stdout_utf8(self) -> Result<String, ShellError> {
        let mut string = String::new();
        {
            let mut lock = self.0.write().unwrap();
            let lock = lock.as_mut().ok_or(ShellError::NoSuchProcess)?;
            lock.child.stdout.as_mut().unwrap().read_to_string(&mut string)?;
        }
        self.wait()?;
        Ok(string)
    }
}
