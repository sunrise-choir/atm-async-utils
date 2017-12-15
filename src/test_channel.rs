//! An in-memory channel, primarily for testing purposes. Like futures::unsync::mpsc, but:
//! - single-producer
//! - allocates a vector of the full capacity in `new`
//! - sender can trigger an error on the receiver
//! - allow an arbitrary SinkError type, for use with `TestSink`

use std::marker::PhantomData;
use std::rc::Rc;
use std::cell::RefCell;

use futures::{Sink, Stream};
use futures::task::Task;

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

    let mut data: Vec<I> = Vec::with_capacity(capacity);

    let channel = Rc::new(RefCell::new(TestChannel {
                                           data,
                                           read: 0,
                                           amount: 0,
                                           task: None,
                                           error: None,
                                           sink_error: PhantomData,
                                       }));

    (Sender(Rc::clone(&channel)), Receiver(channel))
}

struct TestChannel<I, SE, E> {
    data: Vec<I>,
    // reading resumes from this position, this always points into the buffer
    read: usize,
    // amount of valid data
    amount: usize,
    task: Option<Task>,
    error: Option<E>,
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

impl<I, SE, E> Drop for Sender<I, SE, E> {
    fn drop(&mut self) {
        self.0.borrow_mut().unpark();
    }
}

// TODO impl Sink for Sender

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

// TODO impl Stream for Sender
