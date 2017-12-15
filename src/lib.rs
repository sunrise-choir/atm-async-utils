//! Utilities for working with `futures::Sink` and `futures::Stream`.
#![deny(missing_docs)]

extern crate futures;
extern crate quickcheck;

pub mod test_sink;
pub mod test_channel;
