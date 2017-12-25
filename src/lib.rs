//! Utilities for working with `futures::Sink` and `futures::Stream`.
#![deny(missing_docs)]

#[macro_use]
extern crate futures;
extern crate quickcheck;

#[cfg(test)]
extern crate void;
#[cfg(test)]
extern crate rand;

pub mod test_sink;
pub mod test_channel;
pub mod sink_futures;
