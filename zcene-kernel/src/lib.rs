#![feature(lazy_type_alias)]
#![feature(trait_alias)]
#![feature(step_trait)]
#![feature(new_range_api)]
#![feature(allocator_api)]
#![feature(option_zip)]
#![no_std]

////////////////////////////////////////////////////////////////////////////////////////////////////

#[cfg(test)]
#[macro_use]
extern crate std;

////////////////////////////////////////////////////////////////////////////////////////////////////

extern crate alloc;

////////////////////////////////////////////////////////////////////////////////////////////////////

pub mod actor;
pub mod common;
pub mod memory;
pub mod synchronization;
pub mod time;
