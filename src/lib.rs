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

extern crate libc;
extern crate errno;

use std::io::Error;
use std::process::Command;
use errno::Errno;
use errno::errno;
use std::os::unix::process::CommandExt;

#[derive(Debug)]
pub enum ShellError {
    Code(u8),
    Signaled(i32),
    OtherError(Error),
    Errno(&'static str, Errno),
}

impl std::convert::From<std::io::Error> for ShellError {
    fn from(error: std::io::Error) -> ShellError {
        ShellError::OtherError(error)
    }
}

pub type ShellResult = std::result::Result<(), ShellError>;

fn check_errno(name: &'static str, 
               result: libc::c_int) -> Result<libc::c_int, ShellError> {
    if result != -1 {
        Ok(result)
    } else {
        Err(ShellError::Errno(name, errno()))
    }
}

trait ShellResultExt {
    fn code(&self) -> u8;
}

impl ShellResultExt for ShellResult {
    fn code(&self) -> u8 {
        match self {
            &Ok(_) => 0,
            &Err(ShellError::Code(code)) => code,
            &Err(_) => 1
        }
    }
}

pub fn subshell<F>(func: F) ->
        SubShell where F: FnMut() -> ShellResult + 'static {
    SubShell(Box::new(func))
}


/// Something which can be used as a command
pub trait JobSpec where Self: Sized {
    fn exec(self) -> !;

    fn spawn(self) -> Result<JobHandle, ShellError> {
        self.spawn_internal(false)
    }

    fn spawn_internal(self, foreground: bool) -> Result<JobHandle, ShellError> {
        unsafe {
            let foreground = foreground && libc::tcgetpgrp(0) == libc::getpid();
            let pid = check_errno("fork", libc::fork())?;
            // Call setpgid in both processes to avoid race. 
            check_errno("setpgid", libc::setpgid(pid, 0)).unwrap();
            if pid == 0 {
                self.exec()
                // Process replaced
            }
            if foreground {
                check_errno("tcsetpgrp", libc::tcsetpgrp(0, pid));
            }
            Ok(JobHandle { pid: pid })
        }
    }

    fn run(self) -> ShellResult {
        self.spawn_internal(true)?.wait()
    }
}

/// Single Command
pub struct ShellCommand(Command);

impl ShellCommand {
    pub fn new(format: &str, args: &[&str]) -> ShellCommand {
        let mut i = 0;
        let mut vec = format.split(" ").collect::<Vec<_>>();
        for s in vec.iter_mut() {
            if *s == "{}" {
                *s = args[i];
                i += 1;
            }
        }
        let mut command = Command::new(vec[0]);
        command.args(&vec.as_slice()[1..]);
        ShellCommand(command)
    }
}

impl JobSpec for ShellCommand {
    fn exec(mut self) -> ! {
        self.0.exec();
        std::process::exit(1);
    }
}

#[macro_export]
macro_rules! cmd {
    ($format:expr) => ($crate::ShellCommand::new($format, &[]));
    ($format:expr, $($arg:expr),+) => 
        ($crate::ShellCommand::new($format, &[$($arg),+]));
}


/// Block
/// TODO: Change FnMut to FnOnce after fnbox is resolved.
pub struct SubShell(Box<FnMut() -> ShellResult + 'static>);

impl JobSpec for SubShell {
    fn exec(mut self) -> ! {
        let result = self.0();
        if let Err(ref err) = result {
            eprintln!("{:?}", err);
        }
        std::process::exit(result.code() as i32);
    }
}

/// Job which is a process leader.
pub struct JobHandle { pid: i32 }

impl JobHandle {
    /// Sends a SIGTERM to a process group, then wait a process leader.
    pub fn terminate(self) -> ShellResult {
        assert_ne!(self.pid, 0);
        unsafe {
            check_errno("kill", libc::kill(-self.pid, libc::SIGTERM))?;
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
                        return Err(ShellError::Signaled(code));
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

pub fn watch_for_rerun() -> SubShell {
    SubShell(Box::new(move || {
        loop {
            let bin = "foo";
            cmd!("inotifywait -e close_write -r src").run()?;
            if cmd!("cargo build {}", bin).run().is_ok()  {
                break;
            }
        }

        Ok(())
    }))
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
                println!("Start child");
                ::subshell(|| {
                    println!("Start sleeping");
                    ::std::thread::sleep(::std::time::Duration::from_secs(10));
                    println!("Stop sleeping");
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

