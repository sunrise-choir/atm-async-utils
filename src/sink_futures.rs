//! Futures for working with Sinks.

use futures::{Sink, Poll, AsyncSink, Async, Future, Stream};
use futures::stream::Fuse;

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

/// Future which sends all items from a Stream into a Sink. Unlike the tokio SendAll Future, this
/// does not close the sink (it does flush though).
pub struct SendAll<T, U: Stream> {
    sink: Option<T>,
    stream: Option<Fuse<U>>,
    buffered: Option<U::Item>,
}

impl<T, U> SendAll<T, U>
    where T: Sink,
          U: Stream<Item = T::SinkItem>,
          T::SinkError: From<U::Error>
{
    /// Create a new SendAll Future. Unlike the tokio SendAll Future, this does not close the sink
    /// (it does flush though).
    pub fn new(sink: T, stream: U) -> SendAll<T, U> {
        SendAll {
            sink: Some(sink),
            stream: Some(stream.fuse()),
            buffered: None,
        }
    }

    fn sink_mut(&mut self) -> &mut T {
        self.sink
            .as_mut()
            .take()
            .expect("Attempted to poll SendAll after completion")
    }

    fn stream_mut(&mut self) -> &mut Fuse<U> {
        self.stream
            .as_mut()
            .take()
            .expect("Attempted to poll SendAll after completion")
    }

    fn take_result(&mut self) -> (T, U) {
        let sink = self.sink
            .take()
            .expect("Attempted to poll Forward after completion");
        let fuse = self.stream
            .take()
            .expect("Attempted to poll Forward after completion");
        (sink, fuse.into_inner())
    }

    fn try_start_send(&mut self, item: U::Item) -> Poll<(), T::SinkError> {
        debug_assert!(self.buffered.is_none());
        if let AsyncSink::NotReady(item) = self.sink_mut().start_send(item)? {
            self.buffered = Some(item);
            return Ok(Async::NotReady);
        }
        Ok(Async::Ready(()))
    }
}

impl<T, U> Future for SendAll<T, U>
    where T: Sink,
          U: Stream<Item = T::SinkItem>,
          T::SinkError: From<U::Error>
{
    type Item = (T, U);
    type Error = T::SinkError;

    fn poll(&mut self) -> Poll<(T, U), T::SinkError> {
        // If we've got an item buffered already, we need to write it to the
        // sink before we can do anything else
        if let Some(item) = self.buffered.take() {
            try_ready!(self.try_start_send(item))
        }

        loop {
            match self.stream_mut().poll()? {
                Async::Ready(Some(item)) => try_ready!(self.try_start_send(item)),
                Async::Ready(None) => {
                    return Ok(Async::Ready(self.take_result()));
                }
                Async::NotReady => {
                    try_ready!(self.sink_mut().poll_complete());
                    return Ok(Async::NotReady);
                }
            }
        }
    }
}
