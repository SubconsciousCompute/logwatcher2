# Log Watcher 2

A [Rust](https://www.rust-lang.org/) library to watch the log files.

Note: Tested only on Linux

### Features:
1. Automatically reloads log file when log rotated via renaming a log file into another (to be archived) and creation of a blank log file with the same name
2. Calls callback function when new line to parse

### Usage

First, add the following to your `Cargo.toml`

```toml
[dependencies]
logwatcher = "0.2.1"
```

Add to your code,

```rust
use logwatcher::{LogWatcherAction, LogWatcherEvent, LogWatcher};
```

Register the logwatcher, pass a closure and watch it!

```rust
let mut log_watcher = LogWatcher::register("/var/log/auth.log").unwrap();

log_watcher.watch(&mut move |result| {
    match result {
        Ok(event) => match event {
            LogWatcherEvent::Line(line) => {
                println!("Line {}", line);
            }
            LogWatcherEvent::LogRotation => {
                println!("Logfile rotation");
            }
        },
        Err(err) => {
            println!("Error {}", err);
        }
    }
    LogWatcherAction::None
});
```
