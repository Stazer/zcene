#![feature(allocator_api)]
#![feature(sync_unsafe_cell)]
#![feature(abi_x86_interrupt)]
#![feature(naked_functions)]
#![feature(stmt_expr_attributes)]
#![no_std]
#![no_main]

////////////////////////////////////////////////////////////////////////////////////////////////////

extern crate alloc;

////////////////////////////////////////////////////////////////////////////////////////////////////

mod entry_point;
mod global_allocator;
mod kernel;
mod logger;
mod panic_handler;
pub mod actor;
pub mod architecture;
pub mod common;
pub mod driver;
pub mod future;
pub mod memory;
