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
    }).setpgid().spawn().unwrap().wait().unwrap();
}

#[test]
fn test_subshell_kill() {
    let job = shell::subshell(|| {
        cmd!("sleep 3").run()
    }).spawn().unwrap();
    cmd!("sleep 1").run().unwrap();
    // Stop outputting process group.
    assert!(cmd!("pgrep sleep").run().is_ok());
    job.terminate().unwrap();
    assert!(cmd!("pgrep sleep").run().is_err());
}

#[test]
fn test_subshell_setsid_kill() {
    let job = shell::subshell(|| {
        cmd!("sleep 3").run()
    }).setpgid().spawn().unwrap();
    cmd!("sleep 1").run().unwrap();
    // Stop outputting process group.
    assert!(cmd!("pgrep sleep").run().is_ok());
    job.terminate().unwrap();
    assert!(cmd!("pgrep sleep").run().is_err());
}

#[test]
fn test_kill_all_after_wait() {
    let job = shell::subshell(|| {
        cmd!("sleep 1").run()?;
        cmd!("sleep 5").run()?;
        Ok(())
    }).spawn().unwrap();
    cmd!("sleep 2").run().unwrap();
    job.terminate().unwrap();
}

