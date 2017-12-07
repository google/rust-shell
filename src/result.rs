extern crate errno;
extern crate libc;

use errno::Errno;
use errno::errno;
use std::convert::From;
use std::env;
use std::io;

#[derive(Debug)]
pub enum ShellError {
    Code(u8),
    Signaled(i32),
    IoError(io::Error),
    VarError(env::VarError),
    Errno(&'static str, Errno),
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

pub type ShellResult = ::std::result::Result<(), ShellError>;

pub fn check_errno(name: &'static str, 
               result: libc::c_int) -> Result<libc::c_int, ShellError> {
    if result != -1 {
        Ok(result)
    } else {
        Err(ShellError::Errno(name, errno()))
    }
}

pub trait ShellResultExt {
    fn code(&self) -> u8;
    fn print_error(self);
}

impl <T> ShellResultExt for ::std::result::Result<T, ShellError> {
    fn code(&self) -> u8 {
        match self {
            &Ok(_) => 0,
            &Err(ShellError::Code(code)) => code,
            &Err(_) => 1
        }
    }

    fn print_error(self) {
        match self {
            Ok(_) => return,
            Err(err) => { eprintln!("{:?}", err); }
        }
    }
}
