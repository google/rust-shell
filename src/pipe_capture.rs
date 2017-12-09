use libc;
use std::mem;
use libc::c_int;
use result::ShellError;
use result::ShellResult;
use ::result;

pub struct PipeCapture {
    read_pipe: Option<c_int>,
    write_pipe: Option<c_int>
}

impl PipeCapture {
    pub fn new() -> Result<Self, ShellError> {
        let mut pipe = [0 as c_int, 2];
        unsafe {
            result::check_errno("pipe",
                                libc::pipe(&mut pipe[0] as *mut c_int))?;
        }
        Ok(PipeCapture {
            read_pipe: Some(pipe[0]),
            write_pipe: Some(pipe[1])
        })
    }

    fn close_silently(fd: &mut Option<c_int>) {
        if let &mut Some(fd) = fd {
            unsafe {
                libc::close(fd);
            }
        }
        *fd = None;
    }

    pub fn start_reading(&mut self) {
        Self::close_silently(&mut self.write_pipe);
    }

    pub fn start_writing(mut self) -> ShellResult {
        Self::close_silently(&mut self.read_pipe);
        let pipe = mem::replace(&mut self.write_pipe, None);
        unsafe {
            result::check_errno("dup2", libc::dup2(1, pipe.unwrap()))?;
        }
        Ok(())
    }

    fn drain() -> String {
        unimplemented!();
    }
}

impl Drop for PipeCapture {
    fn drop(&mut self) {
        Self::close_silently(&mut self.read_pipe);
        Self::close_silently(&mut self.write_pipe);
    }
}
