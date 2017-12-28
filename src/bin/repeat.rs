// Copyright 2017 Google Inc.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     https://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! This is a recurring task of developping rust shell.  The binary generates
//! README.md from lib.rs's module comments, runs test with preferred option,
//! generates docs, then watches file changes by using inotifywait command.

#[macro_use] extern crate shell;

use shell::ShellResult;
use shell::ShellResultExt;
use std::env;
use std::fs::File;
use std::io;
use std::io::BufRead;
use std::io::BufReader;
use std::io::Write;
use std::os::unix::process::CommandExt;

fn create_readme() -> io::Result<String> {
    let file = File::open("src/lib.rs")?;
    let file = BufReader::new(file);

    let mut codeblock = false;
    let mut readme = String::new();
    for line in file.lines() {
        if line.is_err() {
            break;
        }
        let line = line.unwrap();
        if !line.starts_with("//!") {
            continue;
        }
        let line = if line.len() > 3 { &line[4..] } else { "" };
        if codeblock && line.starts_with("# ") {
            continue;
        }
        if line.starts_with("```") {
            codeblock = !codeblock;
        }
        readme.push_str(line);
        readme.push('\n');
    }
    Ok(readme)
}

fn write_readme(readme: &str) -> io::Result<()> {
    let mut file = File::create("README.md")?;
    file.write_all(readme.as_bytes())
}

fn step() -> ShellResult {
    write_readme(&create_readme()?)?;
    cmd!("cargo test -- --test-threads=1").run()?;
    cmd!("cargo doc").run()
}

fn main() {
    env::set_var("RUST_LOG", "shell=debug");
    step().status().unwrap();

    loop {
        cmd!("inotifywait -e close_write -r src").run().unwrap();
        if cmd!("cargo build").run().status().unwrap().success() {
            break;
        }
    }

    println!("Reload repeat command");
    cmd!("cargo run --bin repeat").command.exec();
}
