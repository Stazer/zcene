#![feature(abi_x86_interrupt)]
#![feature(adt_const_params)]
#![feature(allocator_api)]
#![feature(box_as_ptr)]
#![feature(box_uninit_write)]
#![feature(lazy_type_alias)]
#![feature(naked_functions)]
#![feature(new_range_api)]
#![feature(non_lifetime_binders)]
#![feature(option_zip)]
#![feature(step_trait)]
#![feature(stmt_expr_attributes)]
#![feature(sync_unsafe_cell)]
#![feature(trait_alias)]
#![feature(unsized_const_params)]
#![no_std]
#![no_main]

////////////////////////////////////////////////////////////////////////////////////////////////////

////////////////////////////////////////////////////////////////////////////////////////////////////

//#[cfg(test)]
//#[macro_use]
//extern crate std;

////////////////////////////////////////////////////////////////////////////////////////////////////

extern crate alloc;

////////////////////////////////////////////////////////////////////////////////////////////////////

pub mod memory;
pub mod synchronization;
pub mod time;
pub mod actor;
pub mod architecture;
pub mod driver;
pub mod common;
mod kernel;
