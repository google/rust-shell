use ::Executable;
use job_handle::JobHandle;
use result::ShellResult;
use result::ShellError;
use std::path::Path;
use ::process_manager::ProcessManager;
use std::mem;

#[derive(Debug)]
pub enum Redirect {
    Inherit,
    Capture,
}

#[derive(Debug)]
struct JobSpecData {
    executable: Box<Executable>,
    process_group: bool,
    stdin: Redirect,
    stdout: Redirect,
    stderr: Redirect
}

pub struct JobSpec(Option<JobSpecData>);

impl JobSpec {
    pub fn new<T>(executable: T) -> JobSpec where T : Executable + 'static {
        JobSpec(Some(JobSpecData {
            executable: Box::new(executable),
            process_group: false,
            stdin: Redirect::Inherit,
            stdout: Redirect::Inherit,
            stderr: Redirect::Inherit
        }))
    }

    pub fn process_group(mut self) -> Self {
        self.0.as_mut().unwrap().process_group = true;
        self
    }

    pub fn current_dir(self, _path: &Path) -> Self {
        unimplemented!()
    }

    pub fn env(self, _name: &str, _value: &str) -> Self {
        unimplemented!()
    }

    pub fn stdout(&mut self, redirect: Redirect) {
        self.0.as_mut().unwrap().stdout = redirect;
    }

    pub fn run(self) -> ShellResult {
        self.spawn().and_then(|job| job.wait())
    }

    pub fn spawn(mut self) -> Result<JobHandle, ShellError> {
        self.inner_spawn()
    }

    fn inner_spawn(&mut self) -> Result<JobHandle, ShellError> {
        let data = match mem::replace(&mut self.0, None) {
            Some(data) => data,
            None => {
                return Err(ShellError::InvalidExecutable);
            }
        };
        ProcessManager::fork(data.executable, data.process_group, data.stdin,
                            data.stdout, data.stderr)
    }
}

impl Drop for JobSpec {
    fn drop(&mut self) {
        if self.0.is_none() {
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
