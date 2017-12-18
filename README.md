# rust-shell

Helpers to write shell-script-like tasks in Rust.
                                                                                
## Usage

### cmd!

```rust
cmd!("echo OK").run()?;
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
handle.kill();
```

### Signal dispatching

```
// Start trapping signal for signal dispatching
shell::delegate_signal()?;
```

## Licence

Apatch 2

