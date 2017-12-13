extern crate errno;
extern crate libc;

use errno::Errno;
use errno::errno;
use std::convert::From;
use std::env;
use std::io;
use std::process::ExitStatus;
use std::os::unix::process::ExitStatusExt;

#[derive(Debug)]
pub enum ShellError {
    Status(ExitStatus),
    IoError(io::Error),
    VarError(env::VarError),
    Errno(&'static str, Errno),
    NoSuchProcess,
}

impl ShellError {
    pub fn from_signal(signal: u8) -> Self {
        ShellError::Status(ExitStatus::from_raw(128 + signal as i32))
    }
}

impl From<io::Error> for ShellError {
    fn from(error: io::Error) -> ShellError {
        ShellError::IoError(error)
    }
}

impl From<env::VarError> for ShellError {
    fn from(error: env::VarError) -> ShellError {
        ShellError::VarError(error)
    }
}

pub type ShellResult = Result<(), ShellError>;

pub fn check_errno(name: &'static str,
               result: libc::c_int) -> Result<libc::c_int, ShellError> {
    if result != -1 {
        Ok(result)
    } else {
        Err(ShellError::Errno(name, errno()))
    }
}

pub trait ShellResultExt {
    fn from_status(status: ExitStatus) -> Result<(), ShellError>;
    fn status(self) -> Result<ExitStatus, ShellError>;
    fn code(&self) -> u8;
    fn print_error(self);
}

impl ShellResultExt for Result<(), ShellError> {
    fn from_status(status: ExitStatus) -> Result<(), ShellError> {
        if status.success() {
            Ok(())
        } else {
            Err(ShellError::Status(status))
        }
    }

    fn status(self) -> Result<ExitStatus, ShellError> {
        match self {
            Ok(_) => Ok(ExitStatus::from_raw(0)),
            Err(ShellError::Status(status)) => Ok(status),
            Err(error) => Err(error)
        }
    }

    fn code(&self) -> u8 {
        match self {
            &Ok(_) => 0,
            &Err(ShellError::Status(ref status)) => {
                status.code().unwrap_or(1) as u8
            },
            &Err(_) => 1
        }
    }

    fn print_error(self) {
        match self {
            Ok(_) => return,
            Err(err) => {
                info!("Shell error {:?}", err);
            }
        }
    }
}

#[test]
fn test_from_raw() {
    let s = ExitStatus::from_raw(128 + 15);
    assert_eq!(s.signal().unwrap(), 15);
}
