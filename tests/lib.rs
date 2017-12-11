#[macro_use] extern crate shell;
extern crate libc;

use std::thread;
use std::time::Duration;
use shell::result::ShellResult;
use std::env;

#[test]
fn test_command_run() {
    cmd!("test 0 = 0").run().unwrap();
}

#[test]
fn test_subshell_kill_child() {
    env::set_var("RUST_LOG", "debug");
    let job = shell::spawn(|| -> ShellResult {
        cmd!("sleep 3").run()
    });
    thread::sleep(Duration::from_millis(100));
    // Stop outputting process group.
    assert!(cmd!("pgrep sleep").run().is_ok());
    assert!(job.terminate().is_ok());
    assert!(cmd!("pgrep sleep").run().is_err());
}

#[test]
fn test_kill_all_after_wait() {
    let job = shell::spawn(|| -> ShellResult {
        cmd!("sleep 0.05").run()?;
        cmd!("sleep 2").run()?;
        Ok(())
    });
    thread::sleep(Duration::from_millis(100));
    assert!(job.terminate().unwrap().is_err());
}

#[test]
fn test_kill_thread_job() {
    let job = shell::spawn(|| -> ShellResult {
        cmd!("sleep 5").run()?;
        Ok(())
    });
    thread::sleep(Duration::from_millis(100));
    assert!(job.terminate().unwrap().is_err());
}

#[test]
fn test_signal_before_run() {
    let job = shell::spawn(|| -> ShellResult {
        thread::sleep(Duration::from_millis(100));
        cmd!("sleep 1").run()?;
        Ok(())
    });
    assert!(job.terminate().unwrap().is_err());
}
