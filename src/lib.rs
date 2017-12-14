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
//! run(). async() returns ShellChild which you can use to kill or wait the
//! running process. ShellChild automatically invokes wait() when it's dropped.
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
//! # Threading
//!
//! You can create a subshell by spawning a new thread.
//!
//! ```
//! #[macro_use] extern crate shell;
//! # use shell::result::ShellResult;
//! # fn main() {
//! let job = shell::spawn(|| -> ShellResult {
//!   cmd!("sleep 3").run()?;
//!   Ok(())
//! });
//!
//! job.terminate().unwrap().is_err();
//! # }
//! ```
//!

#[macro_use] extern crate lazy_static;
#[macro_use] extern crate log;
#[macro_use] extern crate nom;
extern crate libc;
extern crate errno;

#[macro_use] pub mod command;
mod shell_child;
mod job_spec;
mod process_manager;
mod local_shell;
pub mod result;

pub use local_shell::spawn;
pub use process_manager::delegate_signal;
