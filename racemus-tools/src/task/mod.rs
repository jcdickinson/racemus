use async_std::sync::{channel, Sender};
use std::future::Future;

pub async fn wait<T, F, FF>(f: F) -> Option<T>
where
    F: FnOnce(Sender<T>) -> FF,
    FF: Future<Output = ()>,
{
    let (tx, rx) = channel(1);
    f(tx).await;
    rx.recv().await
}
