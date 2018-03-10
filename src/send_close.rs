use futures_core::{Future, Poll};
use futures_core::task::Context;
use futures_sink::Sink;
use futures_util::{SinkExt, FutureExt};
use futures_util::future::AndThen;
use futures_util::sink::{close, Close, Send};

/// Future which sends a value down a sink and then closes it.
pub struct SendClose<S: Sink>(AndThen<Send<S>, Close<S>, fn(S) -> Close<S>>);

impl<S: Sink> SendClose<S> {
    /// Create a new `SendClose` future that sends the given `Item` and then closes the sink.
    pub fn new(sink: S, item: S::SinkItem) -> SendClose<S> {
        SendClose(sink.send(item).and_then(|sink| close(sink)))
    }
}

impl<S: Sink> Future for SendClose<S> {
    type Item = S;
    type Error = S::SinkError;

    fn poll(&mut self, cx: &mut Context) -> Poll<S, S::SinkError> {
        self.0.poll(cx)
    }
}
