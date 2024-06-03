use std::fs::File;
use std::io;
use std::io::prelude::*;
use std::io::BufReader;
use std::io::ErrorKind;
use std::io::SeekFrom;
use std::os::unix::fs::MetadataExt;
use std::path::Path;
use std::thread::sleep;
use std::time::Duration;

pub use std::io::Error as LogWatcherError;

pub enum LogWatcherEvent {
    Line(String),
    LogRotation,
}

pub enum LogWatcherAction {
    None,
    Finish,
    SeekToEnd,
}

pub struct LogWatcher {
    filename: String,
    inode: u64,
    pos: u64,
    reader: BufReader<File>,
    finish: bool,
}

impl LogWatcher {
    pub fn register<P: AsRef<Path>>(filename: P) -> Result<LogWatcher, io::Error> {
        let f = match File::open(&filename) {
            Ok(x) => x,
            Err(err) => return Err(err),
        };

        let metadata = match f.metadata() {
            Ok(x) => x,
            Err(err) => return Err(err),
        };

        let mut reader = BufReader::new(f);
        let pos = metadata.len();
        reader.seek(SeekFrom::Start(pos))?;
        Ok(LogWatcher {
            filename: filename.as_ref().to_string_lossy().to_string(),
            inode: metadata.ino(),
            pos,
            reader,
            finish: false,
        })
    }

    fn reopen_if_log_rotated(&mut self) -> bool {
        loop {
            match File::open(&self.filename) {
                Ok(f) => {
                    let metadata = match f.metadata() {
                        Ok(m) => m,
                        Err(_) => {
                            sleep(Duration::new(1, 0));
                            continue;
                        }
                    };
                    if metadata.ino() != self.inode {
                        self.pos = 0;
                        self.reader = BufReader::new(f);
                        self.inode = metadata.ino();
                        return true;
                    } else {
                        sleep(Duration::new(1, 0));
                    }
                    return false;
                }
                Err(err) => {
                    if err.kind() == ErrorKind::NotFound {
                        sleep(Duration::new(1, 0));
                        continue;
                    }
                }
            };
        }
    }

    fn handle_callback_action(&mut self, action: LogWatcherAction) {
        match action {
            LogWatcherAction::SeekToEnd => {
                self.reader.seek(SeekFrom::End(0)).unwrap();
            }
            LogWatcherAction::Finish => {
                self.finish = true;
            }
            LogWatcherAction::None => {}
        }
    }

    pub fn watch<F: ?Sized>(&mut self, callback: &mut F)
    where
        F: FnMut(Result<LogWatcherEvent, LogWatcherError>) -> LogWatcherAction,
    {
        let mut line = String::new();
        loop {
            if self.finish {
                break;
            }
            let resp = self.reader.read_line(&mut line);
            match resp {
                Ok(len) => {
                    if len > 0 {
                        self.pos += len as u64;
                        self.reader.seek(SeekFrom::Start(self.pos)).unwrap();
                        let event = LogWatcherEvent::Line(line.replace('\n', ""));
                        self.handle_callback_action(callback(Ok(event)));
                    } else {
                        if self.reopen_if_log_rotated() {
                            self.handle_callback_action(callback(Ok(LogWatcherEvent::LogRotation)));
                        }
                        self.reader.seek(SeekFrom::Start(self.pos)).unwrap();
                    }
                }
                Err(err) => {
                    self.handle_callback_action(callback(Err(err)));
                }
            }
            line.clear();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::fs;
    use std::path::PathBuf;
    use std::sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    };

    fn logrotation_rename(tmpdir: PathBuf, filename: &str) {
        let log = get_log_path(tmpdir.clone(), filename);
        let mut file = File::create(&log).unwrap();
        sleep(Duration::new(2, 0));
        for _ in 0..10 {
            file.write_all(b"This is a log line\n").unwrap();
        }
        // Rotation
        let mut archived = log.clone();
        archived.pop();
        archived.push(filename);
        archived.set_extension("archive");
        fs::rename(&log, &archived).unwrap();
        // Appending
        let mut file = File::create(&log).unwrap();
        for _ in 0..5 {
            file.write_all(b"This is a rotated log line\n").unwrap();
        }
    }

    fn get_log_path(mut path: PathBuf, filename: &str) -> PathBuf {
        path.push(filename);
        path.set_extension("log");
        path
    }

    #[test]
    fn logwatch_renaming() {
        let tmpdir = env::temp_dir();
        let cloned_tmpdir = tmpdir.clone();
        let filename = "logwatcher2_test";
        let exit = Arc::new(AtomicBool::new(false));
        let exit_clone = exit.clone();

        std::thread::spawn(move || {
            logrotation_rename(cloned_tmpdir, filename);
            exit_clone.store(true, Ordering::SeqCst);
        });
        sleep(Duration::new(1, 0));
        let log = get_log_path(tmpdir.clone(), filename);

        let mut log_watcher = LogWatcher::register(&log).unwrap();
        let mut num_lines = 0;
        let mut rotations = 0;

        log_watcher.watch(&mut |result| {
            match result {
                Ok(event) => match event {
                    LogWatcherEvent::Line(line) => {
                        num_lines += 1;
                        println!("Line {}", line);
                    }
                    LogWatcherEvent::LogRotation => {
                        println!("Logfile rotation");
                        rotations += 1;
                    }
                },
                Err(err) => {
                    println!("Error {}", err);
                }
            }
            if exit.load(Ordering::SeqCst) && num_lines >= 15 {
                LogWatcherAction::Finish
            } else {
                LogWatcherAction::None
            }
        });
        assert_eq!(num_lines, 15);
        assert_eq!(rotations, 1);
    }
}
