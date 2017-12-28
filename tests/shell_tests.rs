// Copyright 2017 Google Inc.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     https://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

#[macro_use] extern crate shell;
extern crate libc;
extern crate env_logger;

use std::thread;
use std::time::Duration;
use shell::ShellResult;
use libc::c_int;

fn setup() {
    env_logger::init().unwrap_or_default();
}

#[test]
fn test_command_run() {
    setup();
    cmd!("test 0 = 0").run().unwrap();
}

#[test]
fn test_subshell_kill_child() {
    setup();
    let job = shell::spawn(|| -> ShellResult {
        cmd!("sleep 3").run()
    });
    thread::sleep(Duration::from_millis(100));
    // Stop outputting process group.
    assert!(cmd!("pgrep sleep").run().is_ok());
    job.signal(libc::SIGTERM);
    assert!(job.join().unwrap().is_err());
    assert!(cmd!("pgrep sleep").run().is_err());
}

#[test]
fn test_kill_all_after_wait() {
    setup();
    let job = shell::spawn(|| -> ShellResult {
        cmd!("sleep 0.05").run()?;
        cmd!("sleep 2").run()
    });
    thread::sleep(Duration::from_millis(100));
    job.signal(libc::SIGTERM);
    assert!(job.join().unwrap().is_err());
}

#[test]
fn test_kill_thread_job() {
    setup();
    let job = shell::spawn(|| -> ShellResult {
        cmd!("sleep 5").run()
    });
    thread::sleep(Duration::from_millis(100));
    job.signal(libc::SIGTERM);
    assert!(job.join().unwrap().is_err());
}

#[test]
fn test_signal_before_run() {
    setup();
    let job = shell::spawn(|| -> ShellResult {
        thread::sleep(Duration::from_millis(100));
        cmd!("sleep 1").run()
    });
    job.signal(libc::SIGTERM);
    assert!(job.join().unwrap().is_err());
}

#[test]
fn test_trap_signal_and_wait_children() {
    setup();
    let result = unsafe {
        let result = libc::fork();
        assert_ne!(result, -1);
        result
    };
    if result == 0 {
        shell::trap_signal_and_wait_children().unwrap();
        unsafe {
            assert_eq!(libc::kill(libc::getpid(), libc::SIGTERM), 0);
        }
        thread::sleep(Duration::from_secs(10));
    } else {
        unsafe {
            let mut status: c_int = 0;
            libc::waitpid(result, &mut status as *mut c_int, 0);
            assert!(libc::WIFEXITED(status));
            assert_eq!(libc::WEXITSTATUS(status), 143);
            assert!(cmd!("pgrep sleep").run().is_err());
        }
    }
}
