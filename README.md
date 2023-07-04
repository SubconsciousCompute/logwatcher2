# Log Watcher 2

A [Rust](https://www.rust-lang.org/) library to watch the log files.

Note: Tested only on Linux

### Features:
1. Automatically reloads log file when log rotated
2. Calls callback function when new line to parse

### Usage

First, add the following to your `Cargo.toml`

```toml
[dependencies]
logwatcher = "0.2.0"
```

Add to your code,

```rust
extern crate logwatcher;
use logwatcher::LogWatcher;
```

Register the logwatcher, pass a closure and watch it!

```rust
let mut log_watcher = LogWatcher::register("/var/log/check.log".to_string()).unwrap();

log_watcher.watch(&mut move |line: String| {
    println!("Line {}", line);
    LogWatcherAction::None
});
```
