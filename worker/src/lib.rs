use std::{
    io,
    process::{ExitStatus, Stdio},
    sync::Arc,
};

use bytes::Bytes;
use futures::{future::FusedFuture, FutureExt, Stream};
use log::error;
use tokio::{
    io::BufReader,
    process::{Child, Command},
    sync::{oneshot, Mutex, Notify, RwLock},
};

mod logs;
mod ops;

async fn process_task(
    mut child: Child,
    inner: Arc<ProcessInner>,
    stop_receiver: oneshot::Receiver<()>,
) {
    // A child process might close both stdout and stderr,
    // but remain alive. In that case, we must still try to wait
    // for the stop message.
    // Fuse the future to be able to .await twice safely (when !is_terminated()).
    let mut stop_receiver = stop_receiver.fuse();

    let stdout = BufReader::new(child.stdout.take().expect("should always be available"));
    let stderr = BufReader::new(child.stderr.take().expect("should always be available"));

    // Phase 1: copy logs from stdout/stderr, on stop message: signal the child.
    tokio::select! {
        copied = futures::future::join(
            logs::copy(stdout, inner.clone()),
            logs::copy(stderr, inner.clone())
        ) => {
            if let Err(e) = copied.0 {
                error!("{:?}", e);
            }
            if let Err(e) = copied.1 {
                error!("{:?}", e);
            }
        },
        _ = &mut stop_receiver => {
            if let Err(e) = ops::stop_child(&mut child).await {
                error!("{:?}", e);
            }
        },
    };

    // Phase 2: stdout/stderr have been closed,
    // wait for a stop message to arrive (if not arrived yet),
    // or on the child to finish otherwise
    loop {
        tokio::select! {
            _ = &mut stop_receiver, if !stop_receiver.is_terminated() => {
                if let Err(e) = ops::stop_child(&mut child).await {
                    error!("{:?}", e);
                }
            },
            res = child.wait() => {
                match res {
                    Ok(s) => {
                        inner.finish(s).await;
                        inner.progress.notify_waiters();
                        return;
                    }
                    Err(_) => unimplemented!("when does this happen?"), // TODO
                }
            }
        }
    }
}

pub(crate) struct ProcessInner {
    exit_status: RwLock<Option<ExitStatus>>,
    logs: RwLock<Vec<Bytes>>,

    // Signals the listeners about progress being made by the process
    // (either new log messages or finishing).
    progress: Arc<Notify>,

    stop_sender: Mutex<Option<oneshot::Sender<()>>>,
}

impl ProcessInner {
    fn new(stop_sender: oneshot::Sender<()>) -> Self {
        Self {
            exit_status: Default::default(),
            logs: Default::default(),
            progress: Default::default(),
            stop_sender: Mutex::new(Some(stop_sender)),
        }
    }
    async fn finish(&self, exit_status: ExitStatus) {
        *self.exit_status.write().await = Some(exit_status);
    }
}

/// Represents a single process.
pub struct Process(Arc<ProcessInner>);

impl Process {
    /// Spawn a new process.
    pub fn spawn<'a>(argv0: &str, argv: impl Iterator<Item = &'a str>) -> Result<Self, io::Error> {
        let (stop_tx, stop_rx) = oneshot::channel();
        let inner = Arc::new(ProcessInner::new(stop_tx));
        let inner_clone = inner.clone();
        let child = Command::new(argv0)
            .args(argv)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        tokio::spawn(process_task(child, inner, stop_rx));

        Ok(Process(inner_clone))
    }

    /// Tries to stop the process, first gracefully by sending SIGTERM,
    /// then, after timeout specified by `SIGTERM_TIMEOUT` elapses, by sending a SIGKILL.
    /// If the process has already finished, returns `Ok` with the exit status.
    /// If the process has not finished,
    /// but another "stop" operation has already been initiated, returns `Err(())`.
    pub async fn stop(&self) -> Result<ExitStatus, ()> {
        if let Some(e) = *self.0.exit_status.read().await {
            return Ok(e);
        }

        match self.0.stop_sender.lock().await.take() {
            Some(tx) => {
                let notify = self.0.progress.clone();

                // Ignore error: if receiver has hung up, process has already finished.
                let _ = tx.send(());

                loop {
                    let notified = notify.notified();
                    if let Some(e) = *self.0.exit_status.read().await {
                        return Ok(e);
                    }
                    notified.await;
                }
            }
            None => Err(()),
        }
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
        logs::stream(self.0.clone())
    }

    /// Gets the `ExitStatus` of the process.
    /// If `None` is returned, the process has not yet finished.
    pub async fn status(&self) -> Option<ExitStatus> {
        *self.0.exit_status.read().await
    }
}

#[cfg(test)]
mod test {
    use std::{os::unix::process::ExitStatusExt, time::Duration};

    use futures::{pin_mut, StreamExt};

    use super::Process;

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
        let logs = p.logs();
        pin_mut!(logs);
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
        let p = Process::spawn("bash", ["-c", script].iter().cloned()).unwrap();
        let logs = p.logs();
        pin_mut!(logs);
        assert_eq!(logs.next().await.as_deref(), Some(&b"hello"[..]));
        assert_eq!(logs.next().await.as_deref(), Some(&b"beautiful"[..]));
        assert_eq!(logs.next().await.as_deref(), Some(&b"world"[..]));
        assert_eq!(logs.next().await, None);
    }

    #[tokio::test]
    async fn test_process_stop() {
        let script = "
            while true; do
                sleep 1
            done;
        ";
        let p = Process::spawn("bash", ["-c", script].iter().cloned()).unwrap();
        assert_eq!(
            p.stop().await.unwrap().signal().unwrap(),
            nix::sys::signal::Signal::SIGTERM as i32
        );

        // Repeated stops return exit status again
        assert_eq!(
            p.stop().await.unwrap().signal().unwrap(),
            nix::sys::signal::Signal::SIGTERM as i32
        );
    }

    #[tokio::test]
    async fn test_process_stop_forceful() {
        let script = r#"
            trap "" TERM
            while true; do
                sleep 1
            done;
        "#;

        let p = Process::spawn("bash", ["-c", script].iter().cloned()).unwrap();

        // Wait until bash executes at least "trap"
        tokio::time::sleep(Duration::from_secs(1)).await;

        assert_eq!(
            p.stop().await.unwrap().signal().unwrap(),
            nix::sys::signal::Signal::SIGKILL as i32
        );
    }
}
