#[macro_use] extern crate shell;

use shell::JobSpec;

fn main() {
    cmd!("cargo test -- --test-threads=1").run().unwrap();
}
