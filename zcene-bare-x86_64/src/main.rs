#![feature(abi_x86_interrupt)]
#![feature(allocator_api)]
#![feature(naked_functions)]
#![feature(stmt_expr_attributes)]
#![feature(sync_unsafe_cell)]
#![no_main]
#![no_std]

////////////////////////////////////////////////////////////////////////////////////////////////////

extern crate alloc;

////////////////////////////////////////////////////////////////////////////////////////////////////

pub mod architecture;
pub mod common;
pub mod driver;
mod entry_point;
mod global_allocator;
mod kernel;
mod panic_handler;
