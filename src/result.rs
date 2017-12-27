extern crate errno;
extern crate libc;

use errno::Errno;
use errno::errno;
use std::convert::From;
use std::default::Default;
use std::env;
use std::io;
use std::marker::PhantomData;
use std::os::unix::process::ExitStatusExt;
use std::process::ExitStatus;

#[derive(Debug)]
pub enum ShellError {
    Status(String, ExitStatus),
    IoError(io::Error),
    VarError(env::VarError),
    Errno(&'static str, Errno),
    NoSuchProcess,
}

impl ShellError {
    pub fn from_signal(command: String, signal: u8) -> Self {
        ShellError::Status(command, ExitStatus::from_raw(128 + signal as i32))
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

pub struct SuccessfulExit(PhantomData<SuccessfulExit>);

impl Default for SuccessfulExit {
    fn default() -> Self {
        SuccessfulExit(PhantomData::default())
    }
}

pub type ShellResult = Result<SuccessfulExit, ShellError>;

pub fn check_errno(name: &'static str,
               result: libc::c_int) -> Result<libc::c_int, ShellError> {
    if result != -1 {
        Ok(result)
    } else {
        Err(ShellError::Errno(name, errno()))
    }
}

pub trait ShellResultExt {
    fn from_status(command: String, status: ExitStatus) -> Self;
    fn status(self) -> Result<ExitStatus, ShellError>;
    fn code(&self) -> u8;
}

impl ShellResultExt for ShellResult {
    fn from_status(command: String, status: ExitStatus)
            -> Self {
        if status.success() {
            Ok(SuccessfulExit(PhantomData::default()))
        } else {
            Err(ShellError::Status(command, status))
        }
    }

    fn status(self) -> Result<ExitStatus, ShellError> {
        match self {
            Ok(_) => Ok(ExitStatus::from_raw(0)),
            Err(ShellError::Status(_, status)) => Ok(status),
            Err(error) => Err(error)
        }
    }

    fn code(&self) -> u8 {
        match self {
            &Ok(_) => 0,
            &Err(ShellError::Status(_, ref status)) => {
                status.code().unwrap_or(1) as u8
            },
            &Err(_) => 1
        }
    }
}

#[test]
fn test_from_raw() {
    let s = ExitStatus::from_raw(128 + 15);
    assert_eq!(s.signal().unwrap(), 15);
}
