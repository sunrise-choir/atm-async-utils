//! Provides a wrapper for sinks, to test blocking and errors.

use futures::{Sink, StartSend, Poll, task, AsyncSink, Async, Stream};
use quickcheck::{empty_shrinker, Arbitrary, Gen};

/// What to do the next time `start_send` is called.
#[derive(Clone, Debug, PartialEq)]
pub enum SendOp<E> {
    /// Simply delegate to the underlying Sink.
    Delegate,

    /// Return `AsyncSink::NotReady` instead of calling into the underlying
    /// operation. The task is immediately notified.
    NotReady,

    /// Return an error instead of calling into the underlying operation.
    Err(E),
}

impl<E> Arbitrary for SendOp<E>
    where E: 'static + Send + Clone
{
    /// Generates 75% Delegate, 25% NotReady.
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        if g.next_f32() < 0.25 {
            SendOp::NotReady
        } else {
            SendOp::Delegate
        }
    }
}

/// What to do the next time `poll_complete` is called.
#[derive(Clone, Debug, PartialEq)]
pub enum FlushOp<E> {
    /// Simply delegate to the underlying Sink.
    Delegate,

    /// Return `Async::NotReady` instead of calling into the underlying
    /// operation. The task is immediately notified.
    NotReady,

    /// Return an error instead of calling into the underlying operation.
    Err(E),
}

impl<E> Arbitrary for FlushOp<E>
    where E: 'static + Send + Clone
{
    /// Generates 75% Delegate, 25% NotReady.
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        if g.next_f32() < 0.25 {
            FlushOp::NotReady
        } else {
            FlushOp::Delegate
        }
    }
}

/// A Sink wrapper that modifies operations of the inner Sink according to the
/// provided iterator.
pub struct TestSink<S: Sink> {
    inner: S,
    send_ops: Box<Iterator<Item = SendOp<S::SinkError>> + Send>,
    flush_ops: Box<Iterator<Item = FlushOp<S::SinkError>> + Send>,
}

impl<S: Sink> TestSink<S> {
    /// Creates a new `TestSink` wrapper over the Sink with the specified `SendOps`s and `FlushOps`.
    pub fn new<I, J>(inner: S, send_iter: I, flush_iter: J) -> Self
        where I: IntoIterator<Item = SendOp<S::SinkError>> + 'static,
              I::IntoIter: Send,
              J: IntoIterator<Item = FlushOp<S::SinkError>> + 'static,
              J::IntoIter: Send
    {
        TestSink {
            inner: inner,
            send_ops: Box::new(send_iter.into_iter().fuse()),
            flush_ops: Box::new(flush_iter.into_iter().fuse()),
        }
    }

    /// Sets the `SendOp`s for this Sink.
    pub fn set_send_ops<I>(&mut self, send_iter: I) -> &mut Self
        where I: IntoIterator<Item = SendOp<S::SinkError>> + 'static,
              I::IntoIter: Send
    {
        self.send_ops = Box::new(send_iter.into_iter().fuse());
        self
    }

    /// Sets the `FlushOp`s for this Sink.
    pub fn set_flush_ops<I>(&mut self, flush_iter: I) -> &mut Self
        where I: IntoIterator<Item = FlushOp<S::SinkError>> + 'static,
              I::IntoIter: Send
    {
        self.flush_ops = Box::new(flush_iter.into_iter().fuse());
        self
    }

    /// Acquires a reference to the underlying Sink.
    pub fn get_ref(&self) -> &S {
        &self.inner
    }

    /// Acquires a mutable reference to the underlying Sink.
    pub fn get_mut(&mut self) -> &mut S {
        &mut self.inner
    }

    /// Consumes this wrapper, returning the underlying Sink.
    pub fn into_inner(self) -> S {
        self.inner
    }
}

impl<S: Sink> Sink for TestSink<S> {
    type SinkItem = S::SinkItem;
    type SinkError = S::SinkError;
    fn start_send(&mut self, item: Self::SinkItem) -> StartSend<Self::SinkItem, Self::SinkError> {
        match self.send_ops.next() {
            Some(SendOp::NotReady) => {
                task::park().unpark();
                Ok(AsyncSink::NotReady(item))
            }
            Some(SendOp::Err(err)) => Err(err),
            Some(SendOp::Delegate) |
            None => self.inner.start_send(item),
        }
    }

    fn poll_complete(&mut self) -> Poll<(), Self::SinkError> {
        match self.flush_ops.next() {
            Some(FlushOp::NotReady) => {
                task::park().unpark();
                Ok(Async::NotReady)
            }
            Some(FlushOp::Err(err)) => Err(err),
            Some(FlushOp::Delegate) |
            None => self.inner.poll_complete(),
        }
    }

    fn close(&mut self) -> Poll<(), Self::SinkError> {
        self.inner.close()
    }
}

impl<S: Sink + Stream> Stream for TestSink<S> {
    type Item = S::Item;
    type Error = S::Error;

    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        self.inner.poll()
    }
}
