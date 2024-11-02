use crate::app::App;
use axum::{
    extract::State,
    response::sse::{Event, Sse},
};
use axum_extra::{headers, TypedHeader};
use futures::{Stream, StreamExt as _};
use std::{convert::Infallible, time::Duration};
use tracing::info;

pub async fn events(
    State(App { sse_event_rx, .. }): State<App>,
    TypedHeader(user_agent): TypedHeader<headers::UserAgent>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    info!("`{}` connected", user_agent.as_str());

    let stream = sse_event_rx.into_stream().map(Ok);

    Sse::new(stream).keep_alive(
        axum::response::sse::KeepAlive::new()
            .interval(Duration::from_secs(1))
            .text("keep-alive-text"),
    )
}
