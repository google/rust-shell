extern crate libc;

use std::time::Duration;

fn main() {
    unsafe {
        let pid = libc::fork();
        if pid == 0 {
            println!("This is a child process {}", pid);
            println!("Before setpgid {}", libc::getpgid(0));
            let result = libc::setpgid(0, 0);
            println!("setpgid = {}", result);
            println!("After setpgid {}", libc::getpgid(0));
            libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGTERM);
            std::thread::sleep(Duration::from_secs(10));
        } else {
            println!("This is a parent process");
            std::thread::sleep(Duration::from_secs(10));
        }
    }
}
