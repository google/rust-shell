#[macro_use] extern crate shell;
extern crate libc;

use shell::JobSpec;
use shell::check_errno;

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
fn test_spawn_background() {
    shell::subshell(|| {
        unsafe {
            let foreground_group = 
                check_errno("tcgetpgrp", libc::tcgetpgrp(0))?;
            let pid = libc::getpid();
            let pgid = check_errno("getpgid", libc::getpgid(0))?;
            println!("pid={} pgid={} foreground={}",
                     pid, pgid, foreground_group);
            assert_eq!(pid, pgid);
            assert_ne!(pgid, foreground_group);
        }
        Ok(())
    }).spawn().unwrap().wait().unwrap();
}

#[test]
fn test_run_foreground() {
    shell::subshell(|| {
        unsafe {
            let foreground_group = 
                check_errno("tcgetpgrp", libc::tcgetpgrp(0))?;
            let pid = libc::getpid();
            let pgid = check_errno("getpgid", libc::getpgid(0))?;
            println!("pid={} pgid={} foreground={}",
                     pid, pgid, foreground_group);
            assert_eq!(pid, pgid);
            assert_eq!(pgid, foreground_group);
        }
        Ok(())
    }).run().unwrap();
}

