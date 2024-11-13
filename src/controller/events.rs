use crate::{shutdown_signal, AppContext};
use axum::{
    extract::State,
    response::sse::{Event, Sse},
};
use axum_extra::{headers, TypedHeader};
use futures::{Stream, StreamExt as _};
use std::{convert::Infallible, time::Duration};
use tracing::info;

pub async fn events<T: AppContext>(
    State(context): State<T>,
    TypedHeader(user_agent): TypedHeader<headers::UserAgent>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    info!("`{}` connected", user_agent.as_str());

    let (_, rx) = context.events().clone();
    let stream = rx.into_stream().map(Ok);
    let stream = or_until_shutdown(stream);

    Sse::new(stream).keep_alive(
        axum::response::sse::KeepAlive::new()
            .interval(Duration::from_secs(1))
            .text("keep-alive-text"),
    )
}

fn or_until_shutdown<S>(stream: S) -> impl Stream<Item = S::Item>
where
    S: Stream,
{
    async_stream::stream! {
        futures::pin_mut!(stream);

        let shutdown_signal = shutdown_signal(None);
        futures::pin_mut!(shutdown_signal);

        loop {
            tokio::select! {
                Some(item) = stream.next() => {
                    yield item
                }
                _ = &mut shutdown_signal => {
                    break;
                }
            }
        }
    }
}
