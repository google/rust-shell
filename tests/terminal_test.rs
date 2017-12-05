#[macro_use] extern crate shell;

use shell::JobSpec;

/// manual test
fn main() {
    shell::try(|| {
        shell::subshell(|| {
            eprintln!("In subshell");
            cmd!("echo OK").run()?;
            Ok(())
        }).spawn()?;
        println!("Reading...");
        cmd!("bash -c read").run()?;
        Ok(())
    }).unwrap();
}
