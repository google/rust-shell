#[macro_use] extern crate shell;

use shell::ShellResult;
use shell::JobSpec;

fn terminal_test() -> ShellResult {
    cmd!("bash -c read").run()?;
    Ok(())
}

fn main() {
    terminal_test().unwrap();
}
