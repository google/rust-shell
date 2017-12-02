//! # Rushell - shell script in rust.
//!
//! Rushell is a library which helps you write shell script like tasks easily
//! in rust.
//!
//! ## Command
//!
//! Command is a spec descripbing everything required to launch a new process.
//! To generate a Command, you can use cmd! macro.
//!
//! ```
//! let command = cmd!("echo test");
//! ```
//!
//! You can specify an argument by using rust value as well.
//!
//! ```
//! let name = "John";
//! let command = cmd!("echo My name is {}.", name);
//! ```
//!
//! ## Running command
//!
//! After creating a command. There are several ways to run it. The simplest
//! way is calling run method of command.
//!
//! ```
//! fn my_shell_script(): ShellError {
//!   cmd!("echo test").run()?;
//!   cmd!("echo test").run()?;
//!   Ok(())
//! }
//! ```
//!
//! Command returns a std::result::Result<(), ShellError>. So you can easily
//! check an error with try operator (?). It ruturns Result::Ok only when
//! command runs successfully and it returns 0 error code.
//!
//! ## Async control
//!
//! If you would like to run a command asynchronously, call async() instead of
//! run(). async() returns JobHandler which you can use to kill or wait the
//! running process. JobHandler automatically invokes wait() when it's dropped.
//! So you will not get a zombi process. You can explicitly detach a process 
//! from job handler by calling detach() if you want to.
//!
//! ```
//! let command = cmd!("sleep 100");
//! let job = command.async();
//! job.wait();
//! ```
//!
//! # Subshell
//!
//! You can create a subshell which is a separated process to run shell
//! command by using subshell() function. subshell() returns a command so
//! that you can call run(), async() as well as a normal external command.
//!
//! ```
//! rushell::subshell(|| {
//!     // Running in a separated process so changing current directory does
//!     // not affect a parante process.
//!     rushell::cd("./hoge")?;
//!     rushell::set_env("ENV_NAME", "HOGE")?;
//!     Ok(())
//! }).run()?;
//! ```
//!


extern crate libc;

use std::io::Error;
use std::process::Command;

pub enum ShellError {
    Code(i32),
    Signal,
    OtherError(Error)
}

impl std::convert::From<std::io::Error> for ShellError {
    fn from(error: std::io::Error) -> ShellError {
        ShellError::OtherError(error)
    }
}

pub type ShellResult = std::result::Result<(), ShellError>;

pub fn subshell<F>(_: F) where F: FnOnce() -> JobHandle {
    let pid = unsafe { libc::fork() };
    if pid == 0 {

    }
}

/// Something which can be used as a command
pub trait CommandLike where Self: Sized {
    fn run(self) -> ShellResult;
    fn async(self) -> JobHandle;
    // TODO: fn output(self) -> ShellOutput;
}

/// Single Command
pub struct ShellCommand(Command);

impl ShellCommand {
    fn new(format: &str, args: &[&str]) -> ShellCommand {
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

impl CommandLike for ShellCommand {
    fn run(mut self) -> ShellResult {
        let status = self.0.status()?;
        if status.success() {
            return Ok(());
        }
        match status.code() {
            Some(code) => Err(ShellError::Code(code)),
            None => Err(ShellError::Signal)
        }
    }

    fn async(self) -> JobHandle {
        unimplemented!();
    }
}

macro_rules! cmd {
    ($format:expr) => (::ShellCommand::new($format, &[]));
    ($format:expr, $($arg:expr),+) => 
        (::ShellCommand::new($format, &[$($arg),+]));
}


/// Block
/// TODO: Change FnMut to FnOnce after fnbox is resolved.
pub struct ShellBlock(Box<FnMut() -> ShellResult + Send + 'static>);

impl CommandLike for ShellBlock {
    fn async(mut self) -> JobHandle {
        unimplemented!();
    }

    fn run(self) -> ShellResult {
        unimplemented!();
    }
}

pub struct JobHandle { pid: usize }

impl JobHandle {
    pub fn kill(self) {
        unsafe { libc::kill(-(self.pid as i32), libc::SIGINT); }
    }
}

impl Drop for JobHandle {
    fn drop(&mut self) {
    }
}

pub fn watch_for_rerun() -> ShellBlock {
    ShellBlock(Box::new(move || {
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
    use ::CommandLike;

    #[test]
    fn it_works() {
        fn body() -> ::ShellResult {
            cmd!("echo Test").run()?;
            Ok(())
        }

        assert!(body().is_ok());

        // let shell = Shell::new();

        // ShellBlock(|shell| {
        //     cmd!("echo hello {}", "hoge").run(shell)?;
        //     let job = cmd!("hoge hoge").async();
        //     job.kill();
        //     Ok(())
        // }).run(shell)?;


        // shell.watch_for_rerun();
        // shell.block(|shell| {
        //   shell.run(cmd!("hoge hoge hoge {}", hoge))?;
        // }).async();
        // shell.run(cmd!("hoge hoge hoge {}", hoge))?;
        //
    }
}

