use ::Executable;
use job_handle::JobHandle;
use result::ShellResult;
use result::ShellError;
use std::path::Path;
use ::signal_handler::SignalHandler;
use std::mem;

pub struct JobSpec {
    executable: Option<Box<Executable>>,
    process_group: bool
}

impl JobSpec {
    pub fn new<T>(executable: T) -> JobSpec where T : Executable + 'static {
        JobSpec {
            executable: Some(Box::new(executable)),
            process_group: false
        }
    }

    pub fn process_group(mut self) -> Self {
        self.process_group = true;
        self
    }

    pub fn current_dir(self, _path: &Path) -> Self {
        unimplemented!()
    }

    pub fn env(self, _name: &str, _value: &str) -> Self {
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
        SignalHandler::fork(executable, self.process_group)
    }
}

impl Drop for JobSpec {
    fn drop(&mut self) {
        if self.executable.is_none() {
            return;
        }
        self.inner_spawn().and_then(|job| job.wait()).unwrap();
    }
}

#[test]
fn test_job_spec_2() {
    use std::process::Command;
    let mut command = Command::new("echo");
    command.arg("The command was run");
    JobSpec::new(command).process_group();
}
