#[macro_use]
extern crate shell;

use std::time::Duration;
use shell::JobSpec;

fn main() {
    cmd!("echo Test2").run().unwrap();
    let job = shell::subshell(|| {
        cmd!("echo Start child").run().unwrap();
        println!("Start child");
        shell::subshell(|| {
            println!("Start sleeping");
            println!("Start sleeping");
            ::std::thread::sleep(::std::time::Duration::from_secs(10));
            println!("Stop sleeping");
            Ok(())
        }).spawn().unwrap();
        Ok(())
    }).spawn().unwrap();
    ::std::thread::sleep(::std::time::Duration::from_secs(1));
    job.terminate().unwrap();
}
