//! An in-memory channel, primarily for testing purposes. Like futures::unsync::mpsc, but:
//! - single-producer
//! - allocates a vector of the full capacity in `new`
//! - sender can trigger an error on the receiver
//! - allow an arbitrary SinkError type, for use with `TestSink`

use std::marker::PhantomData;
use std::rc::Rc;
use std::cell::RefCell;
use std::collections::VecDeque;

use futures::{Sink, Stream, StartSend, Poll, AsyncSink, Async};
use futures::task::{Task, current};

/// Creates a new test channel with the given capacity, and returns a Sink for
/// writing to it, and a Stream for reading from it.
///
///  I: Item, SE: SinkError, E: Error (of the Receiver)
///
/// # Panics
/// Panics if capacity is `0`.
pub fn test_channel<I, SE, E>(capacity: usize) -> (Sender<I, SE, E>, Receiver<I, SE, E>) {
    if capacity == 0 {
        panic!("Invalid test channel capacity.");
    }

    let data: VecDeque<I> = VecDeque::with_capacity(capacity);

    let channel = Rc::new(RefCell::new(TestChannel {
                                           data,
                                           task: None,
                                           error: None,
                                           end_of_stream: false,
                                           sink_error: PhantomData,
                                       }));

    (Sender(Rc::clone(&channel)), Receiver(channel))
}

struct TestChannel<I, SE, E> {
    data: VecDeque<I>,
    task: Option<Task>,
    error: Option<E>,
    end_of_stream: bool,
    sink_error: PhantomData<SE>,
}

impl<I, SE, E> TestChannel<I, SE, E> {
    fn unpark(&mut self) {
        self.task.take().map(|task| task.notify());
    }
}

/// Send data to a TestChannel.
///
/// If there is no space in the channel to hold the item, the current task is
/// parked and notified once space becomes available.
pub struct Sender<I, SE, E>(Rc<RefCell<TestChannel<I, SE, E>>>);

impl<I, SE, E> Sender<I, SE, E> {
    /// Triggers an error on the Receiver.
    pub fn error(&mut self, e: E) {
        let mut c = self.0.borrow_mut();
        c.error = Some(e);
        c.unpark();
    }
}

impl<I, SE, E> Drop for Sender<I, SE, E> {
    fn drop(&mut self) {
        let mut c = self.0.borrow_mut();
        c.end_of_stream = true;
        c.unpark();
    }
}

impl<I, SE, E> Sink for Sender<I, SE, E> {
    type SinkItem = I;
    type SinkError = SE;

    fn start_send(&mut self, item: Self::SinkItem) -> StartSend<Self::SinkItem, Self::SinkError> {
        let mut c = self.0.borrow_mut();

        // Can't write anything, so park.
        if c.data.len() == c.data.capacity() {
            c.task = Some(current());
            return Ok(AsyncSink::NotReady(item));
        } else {
            c.data.push_back(item);
            c.unpark();
            return Ok(AsyncSink::Ready);
        }
    }

    fn poll_complete(&mut self) -> Poll<(), Self::SinkError> {
        Ok(Async::Ready(()))
    }

    fn close(&mut self) -> Poll<(), Self::SinkError> {
        self.0.borrow_mut().end_of_stream = true;
        Ok(Async::Ready(()))
    }
}

/// Receive data from a TestChannel.
///
/// If there is no data in the channel to read from, the current task is parked
/// and notified once data becomes available.
pub struct Receiver<I, SE, E>(Rc<RefCell<TestChannel<I, SE, E>>>);

impl<I, SE, E> Drop for Receiver<I, SE, E> {
    fn drop(&mut self) {
        self.0.borrow_mut().unpark();
    }
}

impl<I, SE, E> Stream for Receiver<I, SE, E> {
    type Item = I;
    type Error = E;

    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        let mut c = self.0.borrow_mut();

        match c.data.pop_front() {
            None => {
                if c.end_of_stream {
                    return Ok(Async::Ready(None));
                }

                let e = c.error.take();
                if e.is_some() {
                    c.unpark();
                    return Err(e.unwrap());
                }

                c.task = Some(current());
                return Ok(Async::NotReady);
            }
            Some(item) => {
                c.unpark();
                return Ok(Async::Ready(Some(item)));
            }
        }
    }
}

#[cfg(test)]
mod tests {

    use quickcheck::{QuickCheck, StdGen};
    use futures::Future;
    use futures::stream::iter_ok;
    use void::Void;
    use rand;

    use super::*;

    #[test]
    fn test_error() {
        let (mut sender, receiver) = test_channel::<u8, u8, u8>(1);
        sender.error(42);
        assert_eq!(receiver.into_future().wait().err().unwrap().0, 42);
    }

    #[test]
    fn test_success() {
        let rng = StdGen::new(rand::thread_rng(), 100);
        let mut quickcheck = QuickCheck::new().gen(rng).tests(10000);
        quickcheck.quickcheck(success as fn(usize, Vec<u8>) -> bool);
    }

    fn success(buf_size: usize, data: Vec<u8>) -> bool {
        let data_copy = data.clone();
        if data.len() == 0 {
            return true;
        }
        let (sender, receiver) = test_channel::<u8, Void, Void>(buf_size + 1);
        let s = sender.send_all(iter_ok(data));

        let r = receiver.collect();
        let (received, _) = r.join(s).wait().unwrap();

        return received == data_copy;
    }
}
