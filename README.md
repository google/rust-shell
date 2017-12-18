# rust-shell

Helpers to write shell-script-like tasks in Rust.
                                                                                
## Usage

### cmd!

```rust
// cmd! parses space-sparated command/arguments list.
cmd!("echo OK").run()?;

// {} is a placeholder.
cmd!("echo {}", "OK").run()?;
cmd!("echo placeholder{}word", "in").run()?;

// Environment variable is automatically extracted
cmd!("echo $HOME/dir").run()?;
```

`cmd!` is a macro generating `ShellCommand`, which is a wrapper for
`std::process::Command`. `ShellCommand#run()` runs a process and wait for an
exit code. If the returned code is zero, `run()` returns `Ok(())`. Otherwise it
returns `Err(ShellError)`.

```
let child = cmd!("sleep 5").spawn()?;
child.wait();
// or child.signal(libc::SIGINT);
```

### shell::spawn

```rust
let handle = shell::spawn(|| -> ShellResult {
  cmd!("sleep 5").run()?;
  Ok(())
});

// Send SIGTERM and wait for exit. Any status code is regarded as a Ok(()).
// System call errors are regarded as an Err().
  handle.terminate()?;
```

### Signal dispatching

```
// Start trapping signal for signal dispatching
shell::delegate_signal()?;

// After calling delegate_signal(), the process delegates a received SIGINT and
// SIGTERM to child processes.
```

## Licence

Apatch 2 Licence

