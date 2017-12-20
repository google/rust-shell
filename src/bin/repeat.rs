//! This is a recurring task of developping rust shell.  The binary runs test
//! with preferred option, generates docs, then watches file changes by using
//! inotifywait command.

#[macro_use] extern crate shell;

use std::env;

fn main() {
    env::set_var("RUST_LOG", "shell=debug");
    loop {
        cmd!("cargo test -- --test-threads=1").run()
            .unwrap_or_default();
        cmd!("cargo doc").run().unwrap();
        cmd!("inotifywait -e close_write -r .").run().unwrap();
    }
}
