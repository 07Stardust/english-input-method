//! 在网络 future 上检查隐私代次，取消时丢弃 future 和迟到结果。

use std::future::Future;
use std::sync::atomic::{AtomicU64, Ordering};
use std::task::Poll;
use std::time::Duration;

pub(crate) async fn run<T>(
    epoch: &AtomicU64,
    expected: u64,
    future: impl Future<Output = T>,
) -> Option<T> {
    let mut future = std::pin::pin!(future);
    let mut tick = tokio::time::interval(Duration::from_millis(5));
    std::future::poll_fn(|context| {
        if epoch.load(Ordering::Acquire) != expected {
            return Poll::Ready(None);
        }
        if let Poll::Ready(result) = future.as_mut().poll(context) {
            return Poll::Ready((epoch.load(Ordering::Acquire) == expected).then_some(result));
        }
        if tick.poll_tick(context).is_ready() {
            context.waker().wake_by_ref();
        }
        Poll::Pending
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cancels_pending_request_and_discards_late_ready_result() {
        let epoch = std::sync::Arc::new(AtomicU64::new(0));
        let cancel = epoch.clone();
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_time()
            .build()
            .unwrap();
        let thread = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(20));
            cancel.fetch_add(1, Ordering::AcqRel);
        });
        assert_eq!(
            runtime.block_on(async {
                tokio::time::timeout(
                    Duration::from_secs(2),
                    run(&epoch, 0, std::future::pending::<u8>()),
                )
                .await
                .unwrap()
            }),
            None
        );
        thread.join().unwrap();
        assert_eq!(
            runtime.block_on(run(&epoch, 0, std::future::ready(7))),
            None
        );
        assert_eq!(
            runtime.block_on(run(&epoch, 1, std::future::ready(7))),
            Some(7)
        );
    }
}
