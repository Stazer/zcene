#![no_std]
#![feature(trait_alias)]
#![feature(allocator_api)]
#![feature(arbitrary_self_types)]
#![feature(fn_traits)]
#![feature(unboxed_closures)]
#![feature(associated_type_defaults)]

////////////////////////////////////////////////////////////////////////////////////////////////////

extern crate alloc;

////////////////////////////////////////////////////////////////////////////////////////////////////

pub mod actor;
pub mod future;
