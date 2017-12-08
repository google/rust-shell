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
//! # use shell::result::ShellResult;
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
//! # use shell::result::ShellResult;
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

#[macro_use] pub mod command;
mod job_spec;
mod job_handle;
mod signal_handler;
pub mod result;

pub use result::check_errno;
use result::ShellResult;
use ::job_spec::JobSpec;

pub trait Executable {
    fn exec(&mut self) -> !;
}

pub fn subshell<F>(func: F) -> JobSpec where F: Fn() -> ShellResult + 'static {
    JobSpec::new(SubShell {
        func: Box::new(func)
    })
}

/// Block
/// TODO: Change FnMut to FnOnce after fnbox is resolved.
pub struct SubShell {
    func: Box<Fn() -> ShellResult + 'static>,
}

impl Executable for SubShell {
    fn exec(&mut self) -> ! {
        (self.func)().unwrap();
        std::process::exit(0);
    }
}

pub fn try<F>(f: F) -> ShellResult where F: FnOnce() -> ShellResult {
    f()
}
