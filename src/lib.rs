//! # Rushell - shell script in rust.
//!
//! Rushell is a helper library for std::process::Command, which allows you to
//! write a shell script helps you to write a shell script in rust.
//!
//! ## cmd! macro
//!
//! You can easiliy create Command instance by using cmd! macro.
//!
//! ```
//! #[macro_use] extern crate shell;
//! fn main() {
//!   let command = cmd!("echo test");
//! }
//! ```
//!
//! You can specify an argument by using rust value as well.
//!
//! ```
//! #[macro_use] extern crate shell;
//! # fn main() {
//!   let name = "John";
//!   let command = cmd!("echo My name is {}.", name);
//! # }
//! ```
//!
//! ## Running command
//!
//! Rushell adds run() method to Command, which returns ShellResult.  Because
//! ShellResult regards exit code 0 is Ok and others are Err, you can easily
//! check an error with try operator (?).
//!
//!
//! ```
//! #[macro_use] extern crate shell;
//! # use shell::ShellResult;
//! # use shell::JobSpec;
//! fn my_shell_script() -> ShellResult {
//!   cmd!("echo test").run()?;
//!   cmd!("echo test").run()?;
//!   Ok(())
//! }
//! # fn main() {
//! #   my_shell_script().unwrap();
//! # }
//! ```
//!
//! ## Output string
//!
//! output_utf8() and error_utf8() can be used to run command and returns
//! String.
//!
//! ## Async control
//!
//! If you would like to run a command asynchronously, call async() instead of
//! run(). async() returns JobHandler which you can use to kill or wait the
//! running process. JobHandler automatically invokes wait() when it's dropped.
//! So you will not get a zombi process. You can explicitly detach a process 
//! from job handler by calling detach() if you want to.
//!
//! ```test
//! #[macro_use] extern crate shell;
//! # use shell::JobSpec;
//! # use shell::ShellResult;
//! # fn main() {
//! # fn body() -> ShellResult {
//! let job = cmd!("sleep 100").spawn()?;
//! job.wait();
//! # Ok(())
//! # }
//! # body();
//! # }
//! ```
//!
//! # Subshell
//!
//! You can create a subshell which is a separated process to run shell
//! command by using subshell() function. subshell() returns a command so
//! that you can call run(), async() as well as a normal external command.
//!
//! ```
//! #[macro_use] extern crate shell;
//! # use shell::JobSpec;
//! # fn main() {
//! shell::subshell(|| {
//!     // Running in a separated process so changing current directory does
//!     // not affect a parante process.
//!     std::env::set_current_dir("./src")?;
//!     std::env::set_var("ENV_NAME", "HOGE");
//!     cmd!("echo test").run()?;
//!     Ok(())
//! }).run().unwrap();
//! # }
//! ```
//!

#[macro_use] extern crate lazy_static;
#[macro_use] extern crate nom;
extern crate libc;
extern crate errno;

mod signal_handler;
#[macro_use] pub mod command;
pub mod result;

pub use result::check_errno;
use result::ShellError;
use result::ShellResult;
use result::ShellResultExt;
use signal_handler::SIGNAL_HANDLER;
use std::panic;
use std::sync::Mutex;

pub fn subshell<F>(func: F) ->
        SubShell where F: Fn() -> ShellResult + 'static {
    SubShell {
        func: Box::new(func),
        setpgid: false
    }
}

/// Something which can be used as a command
pub trait JobSpec where Self: Sized {
    fn exec(self) -> !;
    fn setpgid(self) -> Self;
    fn getpgid(&self) -> bool;

    fn spawn(self) -> Result<JobHandle, ShellError> {
        let pgid = self.getpgid();
        self.spawn_internal(pgid)
    }

    fn spawn_internal(self, setpgid: bool) -> Result<JobHandle, ShellError> {
        let pid = unsafe {
            let mut signal_handler = SIGNAL_HANDLER.lock().unwrap();
            let pid = check_errno("fork", libc::fork())?;
            // Call setpgid in both processes to avoid race. 
            if setpgid {
                check_errno("setpgid", libc::setpgid(pid, 0)).unwrap();
            }
            if pid != 0 {
                signal_handler.add_pid(pid);
            } else {
                signal_handler.clear();
            }
            pid
        };
        unsafe {
            if pid == 0 {
                let mutex = Mutex::new(self);
                panic::catch_unwind(move || {
                    let lock = mutex.into_inner().unwrap();
                    lock.exec()
                }).is_ok();
                std::process::exit(101);
                // Process replaced
            } else {
                libc::kill(pid, libc::SIGCONT);
            }
        }
        // signal_handler.add_pid(pid);
        Ok(JobHandle { pid: pid, setpgid: setpgid })
    }

    fn run(self) -> ShellResult {
        let pgid = self.getpgid();
        self.spawn_internal(pgid)?.wait()
    }
}

/// Block
/// TODO: Change FnMut to FnOnce after fnbox is resolved.
pub struct SubShell {
    func: Box<Fn() -> ShellResult + 'static>,
    setpgid: bool
}

impl JobSpec for SubShell {
    fn exec(mut self) -> ! {
        (self.func)().unwrap();
        std::process::exit(0);
    }

    fn setpgid(mut self) -> Self {
        self.setpgid = true;
        self
    }

    fn getpgid(&self) -> bool {
        return self.setpgid;
    }
}

/// Job which is a process leader.
pub struct JobHandle { pid: i32, setpgid: bool }

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

pub fn try<F>(f: F) -> ShellResult where F: FnOnce() -> ShellResult {
    f()
}

#[cfg(test)]
mod tests {
    use ::JobSpec;

    #[test]
    fn test_command_run() {
        ::try(|| {
            cmd!("test 0 = 0").run()?;
            Ok(())
        }).unwrap();
    }

    #[test]
    fn test_subshell_terminate() {
        ::try(|| {
            let job = ::subshell(|| {
                ::subshell(|| {
                    ::std::thread::sleep(::std::time::Duration::from_secs(10));
                    Ok(())
                }).spawn()?;
                Ok(())
            }).spawn()?;
            ::std::thread::sleep(::std::time::Duration::from_secs(1));
            job.terminate()?;
            Ok(())
        }).unwrap();
    }
}

