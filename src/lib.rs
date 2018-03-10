//! Utilities for working with `futures::Sink` and `futures::Stream`.
#![deny(missing_docs)]

extern crate futures_core;
extern crate futures_sink;
extern crate futures_channel;
extern crate futures_util;
#[cfg(test)]
extern crate futures;

pub mod test_channel;
mod send_close;

pub use send_close::*;
