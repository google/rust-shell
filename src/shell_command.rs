/*
 * Copyright 2017 Google Inc.
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     https://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

use shell_child::ShellChild;
use result::ShellResult;
use result::ShellError;
use std::path::Path;
use std::process::Command;
use std::process::Stdio;

pub struct ShellCommand {
    line: String,
    pub command: Command,
    has_group: bool
}

impl ShellCommand {
    pub fn new(line: String, command: Command) -> ShellCommand {
        ShellCommand {
            line: line,
            command: command,
            has_group: false,
        }
    }

    pub fn set_has_group(mut self) -> Self {
        self.has_group = true;
        self
    }

    pub fn current_dir(self, _path: &Path) -> Self {
        unimplemented!()
    }

    pub fn env(self, _name: &str, _value: &str) -> Self {
        unimplemented!()
    }

    pub fn run(self) -> ShellResult {
        self.spawn().and_then(|job| job.wait())
    }

    pub fn spawn(self) -> Result<ShellChild, ShellError> {
        ShellChild::new(self.line, self.command, self.has_group)
    }

    pub fn stdout_utf8(mut self) -> Result<String, ShellError> {
        self.command.stdout(Stdio::piped());
        self.spawn()?.stdout_utf8()
    }
}

#[test]
fn test_shell_command() {
    assert!(cmd!("test 1 = 1").run().is_ok());
    assert!(cmd!("test 1 = 0").run().is_err());
}

#[test]
fn test_shell_command_output() {
    assert_eq!(&String::from_utf8_lossy(
        &cmd!("echo Test").command.output().unwrap().stdout), "Test\n");
    assert_eq!(cmd!("echo Test").stdout_utf8().unwrap(), "Test\n");
}
