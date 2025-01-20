#![feature(allocator_api)]
#![feature(sync_unsafe_cell)]
#![feature(abi_x86_interrupt)]
#![feature(naked_functions)]
#![no_std]
#![no_main]

////////////////////////////////////////////////////////////////////////////////////////////////////

extern crate alloc;

////////////////////////////////////////////////////////////////////////////////////////////////////

pub mod common;
mod entry_point;
mod future;
mod global_allocator;
mod kernel;
mod logger;
mod panic_handler;
mod smp;
pub mod driver;
