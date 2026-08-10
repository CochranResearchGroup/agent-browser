use super::super::cancellation::CancellationToken;
use std::future::Future;

#[cfg(test)]
mod tests;

/// Stable cancellation error shared by dispatcher and domain action owners.
pub(crate) fn cancellation_error() -> String {
    "Service job was cancelled while running".to_string()
}

/// Await one action effect or the daemon's cooperative cancellation signal.
#[rustfmt::skip]
pub(crate) async fn cancellable<F, T>(future: F, cancellation: Option<CancellationToken>) -> Result<T, String>
where
    F: Future<Output = Result<T, String>>,
{
    let Some(cancellation) = cancellation else {
        return future.await;
    };
    tokio::select! {
        biased;
        _ = cancellation.cancelled() => Err(cancellation_error()),
        result = future => result,
    }
}
