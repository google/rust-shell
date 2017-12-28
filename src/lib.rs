//! # Rust shell - shell script written in rust.
//!
//! Rust shell is a helper library for std::process::Command to write shell
//! script like tasks in rust. The library only works with unix-like operation
//! systems.
//!
//! ## Run command
//!
//! `run!` macro creates a ShellCommand instance which you can run by `run()`
//! method.
//!
//! ```
//! #[macro_use] extern crate shell;
//! 
//! # fn main() {
//! // Run command by cmd! macro
//! cmd!("echo Hello rust shell!").run().unwrap();
//!
//! // Contain white space or non-alphabetical characters
//! cmd!("echo \"%$#\"").run().unwrap();
//!
//! // Pass an argument
//! let name = "shell";
//! cmd!("echo Hello rust {}!", name).run().unwrap();
//!
//! // Extract environment variable
//! cmd!("echo HOME is $HOME").run().unwrap();
//! # }
//! ```
//! ## ShellResult
//!
//! The return value of `ShellCommand#run()` is `ShellResult` which is `Ok(_)`
//! only when the command successfully runs and its execution code is 0, so you
//! can use `?` operator to check if the command successfully exits or not.
//!
//! ```
//! #[macro_use] extern crate shell;
//! use shell::ShellResult;
//!
//! # fn main() {
//! #   shell_function().unwrap();
//! # }
//! fn shell_function() -> ShellResult {
//!   cmd!("echo Command A").run()?;
//!   cmd!("echo Command B").run()?;
//!   shell::ok()
//! }
//! ```
//!
//! ## Output string
//!
//! ShellCommand has a shorthand to obtain stdout as UTF8 string.
//!
//! ```
//! #[macro_use] extern crate shell;
//!
//! # fn main() {
//! assert_eq!(cmd!("echo OK").stdout_utf8().unwrap(), "OK\n");
//! # }
//! ```
//!
//! ## Spawn
//!
//! ShellCommand has `spawn()` method which runs the command asynchronously and
//! returns `ShellChild`.
//!
//! ```
//! #[macro_use] extern crate shell;
//! extern crate libc;
//! use shell::ShellResultExt;
//!
//! # fn main() {
//! // Wait
//! let child = cmd!("sleep 2").spawn().unwrap();
//! child.wait().unwrap();
//!
//! // Signal
//! let child = cmd!("sleep 2").spawn().unwrap();
//! child.signal(libc::SIGINT);
//! let result = child.wait();
//! assert!(result.is_err(), "Should be error as it exits with a signal");
//! assert!(result.status().is_ok(), "Still able to obtain status");
//! # }
//! ```
//!
//! ## Thread
//!
//! If you would like to run a sequence of commands asynchronously,
//! `shell::spawn` creates a thread as well as `std::thread::spawn` but it
//! returns `ShellHandle` wrapping `std::thread::JoinHandle`.
//!
//! `ShellHandle#signal()` is used to send a signal to processes running on the
//! thread.  It also stops launching a new process by `ShellComamnd::run()` on
//! that thread.
//!
//! ```
//! #[macro_use] extern crate shell;
//! extern crate libc;
//! use shell::ShellResult;
//! use shell::ShellResultExt;
//!
//! # fn main() {
//! let handle = shell::spawn(|| -> ShellResult {
//!   cmd!("sleep 3").run()
//! });
//! handle.signal(libc::SIGINT);
//! let result = handle.join().unwrap();
//! assert!(result.is_err(), "Should be error as it exits with a signal");
//! assert!(result.status().is_ok(), "Still able to obtain status");
//! # }
//! ```
//!
//! ## Signal handling
//!
//! `trap_signal_and_wait_children()` starts watching SIGINT and SIGTERM, and
//! waits all child processes before exiting the process when receiving these
//! signals. The function needs to be called before launching any new thread.
//!
//! ```
//! extern crate shell;
//! shell::trap_signal_and_wait_children().unwrap();
//! ```
//!
//! ## Access underlaying objects
//!
//! `ShellComamnd` wraps `std::process::Command` and `ShellChild` wraps
//! `std::process::Child`. Both underlaying objects are accessible via public
//! fields.
//!
//! ```
//! #[macro_use] extern crate shell;
//! use std::process::Stdio;
//! use std::io::Read;
//!
//! # fn main() {
//! // Access std::process::Command.
//! let mut shell_command = cmd!("echo OK");
//! {
//!   let mut command = &mut shell_command.command;
//!   command.stdout(Stdio::piped());
//! }
//!
//! // Access std::process::Child.
//! let shell_child = shell_command.spawn().unwrap();
//! {
//!   let mut lock = shell_child.0.write().unwrap();
//!   let mut child = &mut lock.as_mut().unwrap().child;
//!   let mut str = String::new();
//!   child.stdout.as_mut().unwrap().read_to_string(&mut str);
//! }
//! shell_child.wait().unwrap();
//! # }
//! ```
//!
//! ## License
//! Apatch 2 License

#[macro_use] extern crate lazy_static;
#[macro_use] extern crate log;
#[macro_use] extern crate nom;
extern crate errno;
extern crate libc;
extern crate regex;
extern crate env_logger;

#[macro_use] mod command;
mod shell_child;
mod shell_command;
mod process_manager;
mod local_shell;
mod result;

pub use command::new_command;
pub use local_shell::ShellHandle;
pub use local_shell::spawn;
pub use process_manager::trap_signal_and_wait_children;
pub use result::ShellError;
pub use result::ShellResult;
pub use result::ShellResultExt;
pub use result::ok;
pub use shell_child::ShellChild;
pub use shell_child::ShellChildArc;
pub use shell_child::ShellChildCore;
pub use shell_command::ShellCommand;
