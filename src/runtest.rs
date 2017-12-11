#[macro_use] extern crate shell;

fn main() {
    loop {
        cmd!("cargo test -- --test-threads=1 --nocapture").run().unwrap_or_default();
        cmd!("inotifywait -e close_write -r .").run().unwrap();
    }
}
