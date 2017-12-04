#[macro_use] extern crate shell;

use shell::JobSpec;

/// manual test
fn main() {
    shell::try(|| {
        println!("Reading...");
        cmd!("bash -c read").run()?;
        Ok(())
    }).unwrap();
}
