//! Provides a wrapper for streams, to test blocking and errors.

use futures::{Sink, StartSend, Poll, task, Async, Stream};
use quickcheck::{Arbitrary, Gen};

/// What to do the next time `poll` is called.
#[derive(Clone, Debug, PartialEq)]
pub enum PollOp<E> {
    /// Simply delegate to the underlying Stream.
    Delegate,

    /// Return `AsyncSink::NotReady` instead of calling into the underlying
    /// operation. The task is immediately notified.
    NotReady,

    /// Return an error instead of calling into the underlying operation.
    Err(E),
}

impl<E> Arbitrary for PollOp<E>
    where E: Arbitrary
{
    /// Generates 75% Delegate, 25% NotReady.
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        if g.next_f32() < 0.25 {
            PollOp::NotReady
        } else {
            PollOp::Delegate
        }
    }
}

/// A Stream wrapper that modifies operations of the inner Sink according to the
/// provided iterator.
pub struct TestStream<S: Stream> {
    inner: S,
    poll_ops: Box<Iterator<Item = PollOp<S::Error>> + Send>,
}

impl<S: Stream> TestStream<S> {
    /// Creates a new `TestStream` wrapper over the Samtre with the specified `PollOps`s.
    pub fn new<I>(inner: S, poll_iter: I) -> Self
        where I: IntoIterator<Item = PollOp<S::Error>> + 'static,
              I::IntoIter: Send
    {
        TestStream {
            inner: inner,
            poll_ops: Box::new(poll_iter.into_iter().fuse()),
        }
    }

    /// Sets the `PollOp`s for this Stream.
    pub fn set_poll_ops<I>(&mut self, poll_iter: I) -> &mut Self
        where I: IntoIterator<Item = PollOp<S::Error>> + 'static,
              I::IntoIter: Send
    {
        self.poll_ops = Box::new(poll_iter.into_iter().fuse());
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

impl<S: Stream> Stream for TestStream<S> {
    type Item = S::Item;
    type Error = S::Error;

    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        match self.poll_ops.next() {
            Some(PollOp::NotReady) => {
                task::current().notify();
                Ok(Async::NotReady)
            }
            Some(PollOp::Err(err)) => Err(err),
            Some(PollOp::Delegate) |
            None => self.inner.poll(),
        }
    }
}

impl<S: Sink + Stream> Sink for TestStream<S> {
    type SinkItem = S::SinkItem;
    type SinkError = S::SinkError;

    fn start_send(&mut self, item: Self::SinkItem) -> StartSend<Self::SinkItem, Self::SinkError> {
        self.inner.start_send(item)
    }

    fn poll_complete(&mut self) -> Poll<(), Self::SinkError> {
        self.inner.poll_complete()
    }

    fn close(&mut self) -> Poll<(), Self::SinkError> {
        self.inner.close()
    }
}
