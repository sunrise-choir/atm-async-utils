//! Utilities for working with `futures::Sink` and `futures::Stream`.
#![deny(missing_docs)]

extern crate futures_core;
extern crate futures_sink;
extern crate futures_channel;
#[cfg(test)]
extern crate futures;

pub mod test_channel;
