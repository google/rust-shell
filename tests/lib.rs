#[macro_use] extern crate shell;
extern crate libc;

use shell::check_errno;
use shell::try;
use shell::subshell;
use std::thread;
use std::time::Duration;

#[test]
fn test_command_run() {
    ::try(|| {
        cmd!("test 0 = 0").run()?;
        Ok(())
    }).unwrap();
}

#[test]
fn test_subshell_terminate() {
    println!("test_subshell_terminate started");
    let job = subshell(|| {
        subshell(|| {
            ::std::thread::sleep(::std::time::Duration::from_secs(1000));
            Ok(())
        }).spawn()?;
        Ok(())
    }).spawn().unwrap();
    std::thread::sleep(::std::time::Duration::from_millis(100));
    job.terminate().unwrap();
}

#[test]
fn test_panic_in_run() {
    let result = shell::subshell(|| {
        panic!("Panic");
    }).run();
    assert!(result.is_err());
}

#[test]
fn test_panic_in_spawn() {
    let result = shell::subshell(|| {
        panic!("Panic");
    }).spawn().unwrap().wait();
    assert!(result.is_err());
}

#[test]
fn test_subshell_run() {
    shell::subshell(|| {
        unsafe {
            let foreground_group =
                check_errno("tcgetpgrp", libc::tcgetpgrp(0))?;
            let pid = libc::getpid();
            let pgid = check_errno("getpgid", libc::getpgid(0))?;
            assert_ne!(pid, pgid);
            assert_eq!(pgid, foreground_group);
        }
        Ok(())
    }).run().unwrap();
}

#[test]
fn test_subshell_spawn() {
    shell::subshell(|| {
        unsafe {
            let foreground_group =
                check_errno("tcgetpgrp", libc::tcgetpgrp(0))?;
            let pid = libc::getpid();
            let pgid = check_errno("getpgid", libc::getpgid(0))?;
            assert_ne!(pid, pgid);
            assert_eq!(pgid, foreground_group);
        }
        Ok(())
    }).spawn().unwrap().wait().unwrap();
}

#[test]
fn test_subshell_setpgid_spawn() {
    shell::subshell(|| {
        unsafe {
            let foreground_group =
                check_errno("tcgetpgrp", libc::tcgetpgrp(0))?;
            let pid = libc::getpid();
            let pgid = check_errno("getpgid", libc::getpgid(0))?;
            assert_eq!(pid, pgid);
            assert_ne!(pgid, foreground_group);
        }
        Ok(())
    }).process_group().spawn().unwrap().wait().unwrap();
}

#[test]
fn test_subshell_kill_child() {
    let job = shell::subshell(|| {
        println!("here ?");
        cmd!("sleep 3").run()
    }).spawn().unwrap();
    thread::sleep(Duration::from_millis(100));
    // Stop outputting process group.
    assert!(cmd!("pgrep sleep").run().is_ok());
    job.terminate().unwrap();
    assert!(cmd!("pgrep sleep").run().is_err());
}

#[test]
fn test_subshell_setsid_kill() {
    let job = shell::subshell(|| {
        cmd!("sleep 3").run()
    }).process_group().spawn().unwrap();
    thread::sleep(Duration::from_millis(100));
    // Stop outputting process group.
    assert!(cmd!("pgrep sleep").run().is_ok());
    job.terminate().unwrap();
    assert!(cmd!("pgrep sleep").run().is_err());
}

#[test]
fn test_kill_all_after_wait() {
    let job = shell::subshell(|| {
        cmd!("sleep 0.01").run()?;
        cmd!("sleep 5").run()?;
        Ok(())
    }).spawn().unwrap();
    thread::sleep(Duration::from_millis(100));
    job.terminate().unwrap();
}

#[test]
fn test_kill_thread_job() {
    let id = thread::spawn(|| {
        cmd!("sleep 5");
    });
    thread::sleep(Duration::from_millis(100));
    shell::signal_thread_jobs(&id.thread().id());
    id.join();
}
