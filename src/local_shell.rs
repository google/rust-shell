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

use shell_child::ShellChildArc;
use libc::c_int;
use libc;
use process_manager::PROCESS_MANAGER;
use result::ShellResultExt;
use result::ShellError;
use std::any::Any;
use std::cell::RefCell;
use std::ops::Deref;
use std::sync::Arc;
use std::sync::Mutex;
use std::thread::JoinHandle;
use std::thread::ThreadId;
use std::thread;

/// Thread local shell.
pub struct LocalShell {
    processes: Vec<ShellChildArc>,
    signaled: bool
}

impl LocalShell {
    fn new() -> LocalShell {
        LocalShell {
            processes: Vec::new(),
            signaled: false
        }
    }

    pub fn add_process(&mut self, process: &ShellChildArc) {
        self.processes.push(process.clone());
    }

    pub fn remove_process(&mut self, process: &ShellChildArc) {
        self.processes.retain(|p| !Arc::ptr_eq(p, process));
    }

    pub fn signal(&mut self, signal: c_int) {
        self.signaled = true;
        for process in &self.processes {
            let lock = process.read().unwrap();
            lock.as_ref().ok_or(ShellError::NoSuchProcess)
                .and_then(|p| p.signal(signal)).print_error();
        }
    }

    pub fn wait(&mut self) {
        for process in &self.processes {
            let mut lock = process.write().unwrap();
            lock.take().ok_or(ShellError::NoSuchProcess)
                .and_then(|p| p.wait()).print_error();
        }
    }

    pub fn signaled(&self) -> bool {
        self.signaled
    }
}

struct LocalShellScope(ThreadId, Arc<Mutex<LocalShell>>);

impl LocalShellScope {
    fn new(arc: &Arc<Mutex<LocalShell>>) -> LocalShellScope {
        let mut lock = PROCESS_MANAGER.lock().unwrap();
        let id = thread::current().id();
        lock.add_local_shell(&id, arc);

        LocalShellScope(id, arc.clone())
    }
}

impl Default for LocalShellScope {
    fn default() -> LocalShellScope {
        LocalShellScope::new(&Arc::new(Mutex::new(LocalShell::new())))
    }
}

impl Drop for LocalShellScope {
    fn drop(&mut self) {
        let mut lock = PROCESS_MANAGER.lock().unwrap();
        lock.remove_local_shell(&self.0);
    }
}

pub struct ShellHandle<T> {
    join_handle: JoinHandle<T>,
    shell: Arc<Mutex<LocalShell>>
}

impl <T> ShellHandle<T> {
    pub fn signal(&self, signal: c_int) {
        let mut lock = self.shell.lock().unwrap();
        lock.signal(signal);
    }

    pub fn terminate(self) -> Result<T, Box<Any + Send + 'static>> {
        self.signal(libc::SIGTERM);
        self.join_handle.join()
    }

    pub fn join(self) -> Result<T, Box<Any + Send + 'static>> {
        self.join_handle.join()
    }
}

impl <T> Deref for ShellHandle<T> {
    type Target = JoinHandle<T>;
    fn deref(&self) -> &Self::Target {
        &self.join_handle
    }
}

pub fn spawn<F, T>(f: F) -> ShellHandle<T> where
        F: FnOnce() -> T, F: Send + 'static, T: Send + 'static {
    let arc = Arc::new(Mutex::new(LocalShell::new()));
    let arc_clone = arc.clone();
    let join_handle = thread::spawn(move || -> T {
        LOCAL_SHELL_SCOPE.with(|shell| {
            let mut shell = shell.borrow_mut();
            if shell.is_some() {
                panic!("Shell has already registered");
            }
            *shell = Some(LocalShellScope::new(&arc_clone));
        });
        f()
    });
    ShellHandle {
        join_handle: join_handle,
        shell: arc
    }
}

pub fn current_shell() -> Arc<Mutex<LocalShell>> {
    LOCAL_SHELL_SCOPE.with(|shell| {
        shell.borrow_mut()
            .get_or_insert(LocalShellScope::default()).1.clone()
    })
}

thread_local! {
    static LOCAL_SHELL_SCOPE: RefCell<Option<LocalShellScope>> =
        RefCell::new(None);
}

