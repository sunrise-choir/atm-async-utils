//! An in-memory channel for testing purposes. Allows sending items and errors to a receiver.

use futures_core::{Stream, Poll, Async, Never};
use futures_core::task::Context;
use futures_sink::Sink;
use futures_channel::mpsc::{channel, Sender, Receiver};

/// Create a test channel of a given capacity.
///
/// `I` is the type of items sent over the channel, `E` is the type of errors sent over the channel.
///
/// # Panics
/// Panics if the given capacity is 0.
pub fn test_channel<I, E>(capacity: usize) -> (TestSender<I, E>, TestReceiver<I, E>) {
    if capacity == 0 {
        panic!("TestChannel must have capacity greater than 0")
    }
    let (sender, receiver) = channel(capacity - 1);
    (TestSender::new(sender), TestReceiver::new(receiver))
}

/// The transmission end of a test channel.
///
/// This is built upon `futures::channel::mpcs::sender` and panics if the underlying `Sender` emits
/// an error.
pub struct TestSender<I, E>(Sender<Result<I, E>>);

impl<I, E> TestSender<I, E> {
    fn new(sender: Sender<Result<I, E>>) -> TestSender<I, E> {
        TestSender(sender)
    }
}

impl<I, E> Sink for TestSender<I, E> {
    type SinkItem = Result<I, E>;
    type SinkError = Never;

    fn poll_ready(&mut self, cx: &mut Context) -> Poll<(), Self::SinkError> {
        match self.0.poll_ready(cx) {
            Err(err) => panic!("TestSender got a send error: {:?}", err),
            Ok(non_err) => Ok(non_err),
        }
    }

    fn start_send(&mut self, item: Self::SinkItem) -> Result<(), Self::SinkError> {
        match self.0.start_send(item) {
            Err(err) => panic!("TestSender got a send error: {:?}", err),
            Ok(non_err) => Ok(non_err),
        }
    }

    fn poll_flush(&mut self, cx: &mut Context) -> Poll<(), Self::SinkError> {
        match self.0.poll_flush(cx) {
            Err(err) => panic!("TestSender got a send error: {:?}", err),
            Ok(non_err) => Ok(non_err),
        }
    }

    fn poll_close(&mut self, cx: &mut Context) -> Poll<(), Self::SinkError> {
        match self.0.poll_close(cx) {
            Err(err) => panic!("TestSender got a send error: {:?}", err),
            Ok(non_err) => Ok(non_err),
        }
    }
}

/// The receiving end of a test channel.
pub struct TestReceiver<I, E>(Receiver<Result<I, E>>);

impl<I, E> TestReceiver<I, E> {
    fn new(receiver: Receiver<Result<I, E>>) -> TestReceiver<I, E> {
        TestReceiver(receiver)
    }
}

impl<I, E> Stream for TestReceiver<I, E> {
    type Item = I;
    type Error = E;

    fn poll_next(&mut self, cx: &mut Context) -> Poll<Option<Self::Item>, Self::Error> {
        match self.0.poll_next(cx) {
            Ok(Async::Ready(Some(Ok(item)))) => Ok(Async::Ready(Some(item))),
            Ok(Async::Ready(Some(Err(err)))) => Err(err),
            Ok(Async::Ready(None)) => Ok(Async::Ready(None)),
            Ok(Async::Pending) => Ok(Async::Pending),
            Err(_) => unreachable!(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use futures::{SinkExt, StreamExt, FutureExt};
    use futures::sink::close;
    use futures::stream::iter_ok;
    use futures::executor::block_on;

    #[test]
    fn it_works() {
        let (sender, receiver) = test_channel(2);

        let send_stuff = sender
            .send_all(iter_ok::<_, Never>(vec![Ok(0), Ok(1), Err(0), Ok(2), Err(1)]))
            .and_then(|(sender, _)| close(sender).map(|_| ()));

        let receive_stuff = receiver
            .then(|result| match result {
                      Ok(foo) => Ok(Ok(foo)),
                      Err(err) => Ok(Err(err)),
                  })
            .collect()
            .map(|results| {
                     assert_eq!(results, vec![Ok(0), Ok(1), Err(0), Ok(2), Err(1)]);
                 });

        assert!(block_on(receive_stuff.join(send_stuff)).is_ok());
    }
}
