#![feature(abi_x86_interrupt)]
#![feature(adt_const_params)]
#![feature(allocator_api)]
#![feature(naked_functions)]
#![feature(non_lifetime_binders)]
#![feature(stmt_expr_attributes)]
#![feature(sync_unsafe_cell)]
#![feature(unsized_const_params)]
#![feature(box_as_ptr)]
#![no_main]
#![no_std]

////////////////////////////////////////////////////////////////////////////////////////////////////

extern crate alloc;

////////////////////////////////////////////////////////////////////////////////////////////////////

pub mod actor;
pub mod architecture;
pub mod driver;
mod kernel;
