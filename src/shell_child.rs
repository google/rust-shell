/*
 * Copyright 2017 Google Inc.
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     https://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

use libc::c_int;
use libc;
use local_shell::current_shell;
use result::ShellError;
use result::ShellResult;
use result::ShellResultExt;
use result::check_errno;
use std::io::Read;
use std::mem;
use std::process::Child;
use std::process::Command;
use std::sync::Arc;
use std::sync::RwLock;

#[derive(Debug)]
pub struct ShellChildCore {
    command_line: String,
    pub child: Child,
}

impl ShellChildCore {
    fn new(command_line: String, child: Child) -> ShellChildCore {
        ShellChildCore {
            command_line: command_line,
            child: child,
        }
    }

    pub fn signal(&self, sig: c_int) -> Result<(), ShellError> {
        let kill_pid = self.child.id() as i32;

        info!("Sending signal {} to {}", sig, self.child.id());
        unsafe {
            check_errno("kill", libc::kill(kill_pid, sig))?;
        }
        Ok(())
    }

    pub fn wait_null(&self) -> Result<(), ShellError> {
        unsafe {
            let mut info = mem::uninitialized::<libc::siginfo_t>();
            check_errno("waitid",
                        libc::waitid(
                            libc::P_PID,
                            self.child.id() as u32,
                            &mut info as *mut libc::siginfo_t,
                            libc::WEXITED | libc::WNOWAIT))?;
        }
        Ok(())
    }

    pub fn wait(mut self) -> ShellResult {
        ShellResult::from_status(self.command_line, self.child.wait()?)
    }
}

/// Arc holding `ShellChildCore`.
///
/// This is a combination of the following types.
///
///  - `Arc` to make it accessbile by mutliple threads. (e.g.
///    the thread launched the `ShellChildCore` and the thread sending a signal
///    via `ShellHandle`.
///  - `RwLock` to `signal()` while `wait_null()` is blocking. Both `signal()`
///    and `wait_null()` reguires the read lock which can be obtained by
///    multiple threads at the same time.
///  - `Option` to enable to `take()` ownership of `ShellChildCore` to inovke
///    `wait()`.
pub type ShellChildArc = Arc<RwLock<Option<ShellChildCore>>>;

/// This wraps `ShellChildArc` and provides helper functions.
pub struct ShellChild(pub ShellChildArc);

impl ShellChild {
    pub fn new(line: String, mut command: Command)
            -> Result<ShellChild, ShellError> {
        let shell = current_shell();
        let mut lock = shell.lock().unwrap();
        if lock.signaled() {
            return Err(ShellError::from_signal(line, 101))
        }
        let child = command.spawn()?;
        let process = Arc::new(RwLock::new(
                Some(ShellChildCore::new(line, child))));
        lock.add_process(&process);
        Ok(ShellChild(process))
    }

    /// Sends a signal to the process.
    pub fn signal(&self, signal: c_int) -> Result<(), ShellError> {
        let process = &self.0;
        let process = process.read().unwrap();
        process.as_ref().ok_or(ShellError::NoSuchProcess)?.signal(signal)
    }

    /// Waits for termination of the process.
    pub fn wait(self) -> ShellResult {
        {
            let data = self.0.read().unwrap();
            data.as_ref().ok_or(ShellError::NoSuchProcess)?.wait_null()?;
        }
        let result = {
            let mut data = self.0.write().unwrap();
            data.take().ok_or(ShellError::NoSuchProcess)
                .and_then(|c| c.wait())
        };
        {
            let shell = current_shell();
            let mut lock = shell.lock().unwrap();
            lock.remove_process(&self.0);
        }
        result
    }

    /// Obtains stdout as utf8 string.
    /// Returns Err if it returns non-zero exit code.
    pub fn stdout_utf8(self) -> Result<String, ShellError> {
        let mut string = String::new();
        {
            let mut lock = self.0.write().unwrap();
            let lock = lock.as_mut().ok_or(ShellError::NoSuchProcess)?;
            lock.child.stdout.as_mut().unwrap().read_to_string(&mut string)?;
        }
        self.wait()?;
        Ok(string)
    }
}
