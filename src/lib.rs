use std::path::PathBuf;
use std::io::Error;
use std::io::ErrorKind;
use std::io::Result;
use std::process::Command;
use std::process::ExitStatus;
use std::sync::Arc;
use std::sync::Mutex;

/// Something which can be used as a command
pub trait CommandLike where Self: Sized {
    fn run(self, shell: &Shell) -> Result<()> {
        self.status(shell).and_then(|s| {
            if s.success() {
                Ok(())
            } else {
                Err(Error::new(ErrorKind::Other, "Status code is not 0"))
            }
        })
    }

    fn async(self, shell: &Shell) -> AsyncJobHandle;

    fn status(self, shell: &Shell) -> Result<ExitStatus>;
}

/// Single Command
pub struct ShellCommand(Command);

impl ShellCommand {
    fn new(format: &str, args: &[&str]) -> ShellCommand {
        let mut i = 0;
        let mut vec = format.split(" ").collect::<Vec<_>>();
        for s in vec.iter_mut() {
            if *s == "{}" {
                *s = args[i];
                i += 1;
            }
        }
        let mut command = Command::new(vec[0]);
        command.args(&vec.as_slice()[1..]);
        ShellCommand(command)
    }
}

impl CommandLike for ShellCommand {
    fn status(mut self, shell: &Shell) -> Result<ExitStatus> {
        self.0.status()
    }
    fn async(self, shell: &Shell) -> AsyncJobHandle {
        unimplemented!();
    }
}

macro_rules! cmd {
    ($format:expr) => (ShellCommand::new($format, &[]));
    ($format:expr, $($arg:expr),+) => 
        (ShellCommand::new($format, &[$($arg),+]));
}


/// Block
/// TODO: Change FnMut to FnOnce after fnbox is resolved.
pub struct ShellBlock(Box<FnMut(&mut Shell) -> Result<()> + Send + 'static>);

impl CommandLike for ShellBlock {
    fn async(mut self, shell: &Shell) -> AsyncJobHandle {
        let mut subshell = shell.clone();
        let job = AsyncJobHandle { status: subshell.status.clone() };
        std::thread::spawn(move || {
            self.0(&mut subshell)
        });
        job
    }

    fn status(self, shell: &Shell) -> Result<ExitStatus> {
        unimplemented!();
    }
}

enum ShellStatus {
    NotRunning,
    Running(usize),
    Killed
}

impl ShellStatus {
    fn new() -> Arc<Mutex<ShellStatus>> {
        Arc::new(Mutex::new(ShellStatus::NotRunning))
    }
}

pub struct AsyncJobHandle {
    status: Arc<Mutex<ShellStatus>>
}

impl AsyncJobHandle {
    pub fn kill(self) {
        let mut lock = self.status.lock().expect(
            "Failed to obtain mutex lock");
        match *lock {
            ShellStatus::NotRunning => {}
            ShellStatus::Running(pid) => {}
            ShellStatus::Killed => unreachable!() 
        }
        *lock = ShellStatus::Killed;
    }
}

impl Drop for AsyncJobHandle {
    fn drop(&mut self) {
    }
}

pub struct Shell {
    current_directory: PathBuf,
    status: Arc<Mutex<ShellStatus>>
}

impl Shell {
    pub fn new() -> Shell {
        Shell {
            current_directory: PathBuf::new(),
            status: ShellStatus::new()
        }
    }

    pub fn cd(&mut self) {
        unimplemented!();
    }

    pub fn clone(&self) -> Shell {
        Shell {
            current_directory: self.current_directory.clone(),
            status: ShellStatus::new()
        }
    }

}

pub fn watch_for_rerun() -> ShellBlock {
    ShellBlock(Box::new(move |shell| {
        loop {
            let bin = "foo";
            cmd!("inotifywait -e close_write -r src").run(shell)?;
            if cmd!("cargo build {}", bin).run(shell).is_ok()  {
                break;
            }
        }

        Ok(())
    }))
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let shell = Shell::new();

        ShellBlock(|shell| {
            cmd!("echo hello {}", "hoge").run(shell)?;
            let job = cmd!("hoge hoge").async();
            job.kill();
            Ok(())
        }).run(shell)?;


        // shell.watch_for_rerun();
        // shell.block(|shell| {
        //   shell.run(cmd!("hoge hoge hoge {}", hoge))?;
        // }).async();
        // shell.run(cmd!("hoge hoge hoge {}", hoge))?;
        //
    }
}

