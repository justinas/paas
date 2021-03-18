use std::io::Error as IoError;
use std::sync::Arc;

use bytes::Bytes;
use futures::Stream;
use tokio::io::{AsyncBufRead, AsyncBufReadExt};

use super::ProcessInner;

pub(crate) async fn copy<R: AsyncBufRead + Unpin>(
    reader: R,
    process: Arc<ProcessInner>,
) -> Result<(), IoError> {
    let mut lines = reader.lines();
    // TODO: re-locks each line, not too efficient
    while let Some(line) = lines.next_line().await? {
        process.logs.write().unwrap().push(Bytes::from(line));
        process.progress.notify_waiters();
    }
    Ok(())
}

pub(crate) fn stream(process: Arc<ProcessInner>) -> impl Stream<Item = Bytes> {
    let notify = process.progress.clone();
    let mut pos = 0;
    async_stream::stream! {
        loop {
            let notified = notify.notified();
            let line = {
                // TODO: read multiple lines here for efficiency.
                // We use `bytes::Bytes` and it is internally ref-counted,
                // so perhaps clone `Bytes` objects to a stack buffer,
                // unlock quickly, then yield each object outside of the lock.
                let logs = process.logs.read().unwrap();
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
            let process_finished = process.exit_status.read().unwrap().is_some();
            let has_read_all = pos == process.logs.read().unwrap().len();
            if process_finished && has_read_all {
                return;
            }
            if has_read_all {
                notified.await;
            }
        }
    }
}
