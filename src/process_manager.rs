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
use libc::sigset_t;
use std::sync::Arc;
use std::sync::Mutex;
use std::thread::ThreadId;
use local_shell::LocalShell;
use std::collections::HashMap;
use result::check_errno;
use result::ShellError;
use result::ShellResult;
use std::mem;
use errno::Errno;
use std::thread;

/// Managing global child process state.
pub struct ProcessManager {
    children: HashMap<ThreadId, Arc<Mutex<LocalShell>>>
}

impl ProcessManager {
    fn new() -> ProcessManager {
        ProcessManager {
            children: HashMap::new()
        }
    }

    pub fn add_local_shell(&mut self, id: &ThreadId,
                           shell: &Arc<Mutex<LocalShell>>) {
        self.children.insert(id.clone(), shell.clone());
    }

    pub fn remove_local_shell(&mut self, id: &ThreadId) {
        self.children.remove(id);
    }
}

/// Delegates SIGINT and SIGTERM to child processes.
#[allow(dead_code)]
pub fn delegate_signal() -> ShellResult {
    unsafe {
        let mut sigset = mem::uninitialized::<sigset_t>();
        check_errno("sigemptyset",
                    libc::sigemptyset(&mut sigset as *mut sigset_t))?;
        check_errno("sigaddset", libc::sigaddset(
                &mut sigset as *mut sigset_t, libc::SIGINT))?;
        check_errno("sigaddset", libc::sigaddset(
                &mut sigset as *mut sigset_t, libc::SIGTERM))?;

        let mut oldset = mem::uninitialized::<sigset_t>();
        let result = libc::pthread_sigmask(
            libc::SIG_BLOCK, &mut sigset as *mut sigset_t,
            &mut oldset as *mut sigset_t);
        if result != 0 {
            return Err(ShellError::Errno("pthread_sigmask", Errno(result)));
        }

        thread::spawn(move || {
            info!("Start waitinig signal");
            let mut signal: c_int = 0;
            let result = libc::sigwait(
                &sigset as *const sigset_t, &mut signal as *mut c_int);
            if result != 0 {
                eprintln!("sigwait failed {}", result);
                return;
            }
            info!("Signal {} is received", signal);
            let mut lock = PROCESS_MANAGER.lock().unwrap();
            let mut children = lock.children.drain().collect::<Vec<_>>();
            for &mut (_, ref entry) in &mut children {
                let mut lock = entry.lock().unwrap();
                lock.signal(signal);
            }
            for &mut (_, ref entry) in &mut children {
                let mut lock = entry.lock().unwrap();
                lock.wait();
            }
            ::std::process::exit(128 + signal);
        });
        Ok(())
    }
}

lazy_static! {
    pub static ref PROCESS_MANAGER: Mutex<ProcessManager> =
        Mutex::new(ProcessManager::new());
}
