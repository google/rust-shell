#[macro_use] extern crate shell;

use std::env;

fn main() {
    env::set_var("RUST_LOG", "shell=debug");
    loop {
        cmd!("cargo test -- --test-threads=1 --nocapture").run()
            .unwrap_or_default();
        cmd!("inotifywait -e close_write -r .").run().unwrap();
    }
}
