//! Futures for working with Sinks.

use futures::{Sink, Poll, Async, Future, Stream};
use futures::future::AndThen;
use futures::sink::Send;

/// Future which closes a sink.
pub struct Close<S: Sink> {
    sink: Option<S>,
}

impl<S: Sink> Close<S> {
    /// Create a new `Close` future.
    pub fn new(s: S) -> Close<S> {
        Close { sink: Some(s) }
    }

    /// Get a shared reference to the inner sink.
    pub fn get_ref(&self) -> &S {
        self.sink
            .as_ref()
            .take()
            .expect("Attempted Close::get_ref after completion")
    }

    /// Get a mutable reference to the inner sink.
    pub fn get_mut(&mut self) -> &mut S {
        self.sink
            .as_mut()
            .take()
            .expect("Attempted Close::get_mut after completion")
    }
}

impl<S: Sink> Future for Close<S> {
    type Item = S;
    type Error = S::SinkError;

    fn poll(&mut self) -> Poll<S, S::SinkError> {
        let mut s = self.sink
            .take()
            .expect("Attempted to poll Close after completion");

        match s.close() {
            Ok(Async::Ready(_)) => {
                return Ok(Async::Ready(s));
            }
            Ok(Async::NotReady) => {
                self.sink = Some(s);
                return Ok(Async::NotReady);
            }
            Err(e) => {
                return Err(e);
            }
        }
    }
}

/// Future which sends a value down a sink and then closes it.
pub struct SendClose<S: Sink>(AndThen<Send<S>, Close<S>, fn(S) -> Close<S>>);

impl<S: Sink> SendClose<S> {
    /// Create a new `SendClose` future that sends the given `Item`.
    pub fn new(sink: S, item: S::SinkItem) -> SendClose<S> {
        SendClose(sink.send(item).and_then(|ps_sink| Close::new(ps_sink)))
    }
}

impl<S: Sink> Future for SendClose<S> {
    type Item = S;
    type Error = S::SinkError;

    fn poll(&mut self) -> Poll<S, S::SinkError> {
        self.0.poll()
    }
}
