use std::{io::Error as IoError, process::ExitStatus, sync::Arc};

use bytes::Bytes;
use tokio::{
    process::Command,
    sync::{Notify, RwLock},
};

#[derive(Default)]
struct ProcessInner {
    exit_status: RwLock<Option<ExitStatus>>,
    logs: Vec<Bytes>,
    progress: Notify,
}

impl ProcessInner {
    async fn finish(&self, exit_status: ExitStatus) {
        *self.exit_status.write().await = Some(exit_status);
    }
}

struct Process(Arc<ProcessInner>);

impl Process {
    pub fn spawn<'a>(argv0: &str, argv: impl Iterator<Item = &'a str>) -> Result<Self, IoError> {
        let inner = Arc::new(ProcessInner::default());
        let inner_clone = inner.clone();
        let mut cmd = Command::new(argv0).args(argv).spawn()?;
        tokio::spawn(async move {
            // TODO: read stdin/stdout
            match cmd.wait().await {
                Ok(s) => inner.finish(s).await,
                Err(_) => unimplemented!("when does this happen?"), // TODO
            }
        });
        Ok(Process(inner_clone))
    }

    pub async fn status(&self) -> Option<ExitStatus> {
        *self.0.exit_status.read().await
    }
}

#[cfg(test)]
mod test {
    use super::Process;
    use std::time::Duration;

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
        let p = Process::spawn("true", empty_args()).unwrap();
        tokio::time::sleep(Duration::from_secs(1)).await; // TODO: proper wait
        assert_eq!(p.status().await.unwrap().code(), Some(0));
    }
}
