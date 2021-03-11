use std::{
    io::Error as IoError,
    process::{ExitStatus, Stdio},
    sync::Arc,
};

use bytes::Bytes;
use futures::Stream;
use tokio::{
    io::{AsyncBufRead, AsyncBufReadExt, BufReader},
    process::Command,
    sync::{Notify, RwLock},
};

async fn copy_log<R: AsyncBufRead + Unpin>(
    reader: R,
    process: Arc<ProcessInner>,
) -> Result<(), IoError> {
    let mut lines = reader.lines();
    // TODO: re-locks each line, not too efficient
    while let Some(line) = lines.next_line().await? {
        process.logs.write().await.push(Bytes::from(line));
        process.progress.notify_waiters();
    }
    process.progress.notify_waiters();
    Ok(())
}

#[derive(Default)]
struct ProcessInner {
    exit_status: RwLock<Option<ExitStatus>>,
    logs: RwLock<Vec<Bytes>>,
    // Signals the listeners about progress being made by the process
    // (either new log messages or finishing).
    progress: Arc<Notify>,
}

impl ProcessInner {
    async fn finish(&self, exit_status: ExitStatus) {
        *self.exit_status.write().await = Some(exit_status);
    }
}

/// Represents a single process.
pub struct Process(Arc<ProcessInner>);

impl Process {
    /// Spawn a new process.
    pub fn spawn<'a>(argv0: &str, argv: impl Iterator<Item = &'a str>) -> Result<Self, IoError> {
        let inner = Arc::new(ProcessInner::default());
        let inner_clone = inner.clone();
        let mut child = Command::new(argv0)
            .args(argv)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        tokio::spawn(async move {
            let stdout = BufReader::new(child.stdout.take().expect("should always be available"));
            let stderr = BufReader::new(child.stderr.take().expect("should always be available"));

            let copied = tokio::join!(
                copy_log(stdout, inner.clone()),
                copy_log(stderr, inner.clone())
            );
            if let Err(e) = copied.0 {
                // TODO: use `log` crate
                eprintln!("{:?}", e);
            }
            if let Err(e) = copied.1 {
                // TODO: use `log` crate
                eprintln!("{:?}", e);
            }
            match child.wait().await {
                Ok(s) => {
                    inner.finish(s).await;
                    inner.progress.notify_waiters();
                }
                Err(_) => unimplemented!("when does this happen?"), // TODO
            }
        });

        Ok(Process(inner_clone))
    }

    /// Returns a stream which yields stdout and stderr logs.
    /// Each stream item is a single line.
    /// Each invocation of `logs()` returns an independent stream
    /// that returns a copy of the logs.
    ///
    /// When the stream returns, the process has finished
    /// and it is guaranteed that subsequent calls to `Process::status()`
    /// will return `Some(ExitStatus)`.
    pub fn logs(&self) -> impl Stream<Item = Bytes> {
        let inner = self.0.clone();
        let notify = inner.progress.clone();
        let mut pos = 0;
        Box::pin(async_stream::stream! {
            loop {
                let notified = notify.notified();
                let line = {
                    // TODO: batch
                    let logs = inner.logs.read().await;
                    if pos < logs.len() {
                        let line = logs[pos].clone();
                        pos += 1;
                        Some(line)
                    } else {
                        None
                    }
                };
                if let Some(l) = line {
                    yield l;
                }
                let process_finished = inner.exit_status.read().await.is_some();
                let has_read_all = pos == inner.logs.read().await.len();
                if process_finished && has_read_all {
                    return;
                }
                if has_read_all {
                    notified.await;
                }
            }
        })
    }

    /// Gets the `ExitStatus` of the process.
    /// If `None` is returned, the process has not yet finished.
    pub async fn status(&self) -> Option<ExitStatus> {
        *self.0.exit_status.read().await
    }
}

#[cfg(test)]
mod test {
    use super::Process;
    use futures::StreamExt;

    fn empty_args() -> impl Iterator<Item = &'static str> {
        (&mut []).iter().cloned()
    }

    #[should_panic]
    #[tokio::test]
    async fn test_process_spawn_not_found() {
        Process::spawn("this_command_does_not_exist", empty_args()).unwrap();
    }

    #[tokio::test]
    async fn test_process_spawn() {
        let p = Process::spawn("echo", ["foo"].iter().cloned()).unwrap();
        let mut logs = p.logs();
        assert_eq!(logs.next().await.as_deref(), Some(&b"foo"[..]));
        assert_eq!(logs.next().await, None);
        assert_eq!(p.status().await.unwrap().code(), Some(0));
    }

    #[tokio::test]
    async fn test_process_log_stream() {
        let script = "
            echo hello
            sleep 1
            echo beautiful
            echo world
            sleep 2
        ";
        let p = Process::spawn("/usr/bin/env", ["bash", "-c", &script].iter().cloned()).unwrap();
        let mut logs = p.logs();
        assert_eq!(logs.next().await.as_deref(), Some(&b"hello"[..]));
        assert_eq!(logs.next().await.as_deref(), Some(&b"beautiful"[..]));
        assert_eq!(logs.next().await.as_deref(), Some(&b"world"[..]));
        assert_eq!(logs.next().await, None);
    }
}
