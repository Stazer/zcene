//! Asynchronous mpsc channel.
//!
//! Vendored from `futures-channel` and trimmed to the [`mpsc`] module, made `no_std`, and
//! modified to allocate from a caller-provided `allocator_api` allocator.

#![no_std]
#![feature(allocator_api)]

extern crate alloc;

pub mod mpsc;
