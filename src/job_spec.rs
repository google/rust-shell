use ::Executable;
use ::libc;
use job_handle::JobHandle;
use result::ShellError;
use result::ShellResult;
use result::check_errno;
use signal_handler::SIGNAL_HANDLER;
use std::mem;
use std::panic;
use std::path::Path;
use std::process::Command;
use std::process;
use std::sync::Mutex;

pub struct JobSpec2 {
    executable: Option<Box<Executable>>,
    process_group: bool
}

impl JobSpec2 {
    pub fn new<T>(executable: T) -> JobSpec2 where T : Executable + 'static {
        JobSpec2 {
            executable: Some(Box::new(executable)),
            process_group: false
        }
    }

    pub fn process_group(mut self) -> Self {
        self.process_group = true;
        self
    }

    pub fn current_dir(mut self, path: &Path) -> Self {
        unimplemented!()
    }

    pub fn env(mut self, name: &str, value: &str) -> Self {
        unimplemented!()
    }

    pub fn run(self) -> ShellResult {
        self.spawn().and_then(|job| job.wait())
    }

    pub fn spawn(mut self) -> Result<JobHandle, ShellError> {
        self.inner_spawn()
    }

    fn inner_spawn(&mut self) -> Result<JobHandle, ShellError> {
        let executable = match mem::replace(&mut self.executable, None) {
            Some(executable) => executable,
            None => {
                return Err(ShellError::InvalidExecutable);
            }
        };
        let pid = unsafe {
            let mut signal_handler = SIGNAL_HANDLER.lock().unwrap();
            let pid = check_errno("fork", libc::fork())?;
            // Call setpgid in both processes to avoid race. 
            if self.process_group {
                check_errno("setpgid", libc::setpgid(pid, 0)).unwrap();
            }
            if pid != 0 {
                signal_handler.add_pid(pid);
            } else {
                signal_handler.clear();
            }
            pid
        };
        unsafe {
            if pid == 0 {
                let mutex = Mutex::new(executable);
                panic::catch_unwind(move || {
                    let mut lock = mutex.lock().unwrap();
                    lock.exec()
                }).is_ok();
                // The control reaches here only when the process was paniced.
                process::exit(101);
            } else {
                if self.process_group {
                    libc::kill(pid, libc::SIGCONT);
                }
            }
        }
        Ok(JobHandle { pid: pid, setpgid: self.process_group })
    }
}

impl Drop for JobSpec2 {
    fn drop(&mut self) {
        if self.executable.is_none() {
            return;
        }
        self.inner_spawn().and_then(|job| job.wait()).unwrap();
    }
}

#[test]
fn test_job_spec_2() {
    let mut command = Command::new("echo");
    command.arg("The command was run");
    JobSpec2::new(command).process_group();
}
