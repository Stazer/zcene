#![no_std]
#![feature(trait_alias)]
#![feature(allocator_api)]
#![feature(arbitrary_self_types)]

////////////////////////////////////////////////////////////////////////////////////////////////////

extern crate alloc;

////////////////////////////////////////////////////////////////////////////////////////////////////

mod future_runtime;
mod future_runtime_actor_address;
mod future_runtime_actor_handler;
mod future_runtime_allocator;
mod future_runtime_box_future;
mod future_runtime_concurrent_queue;
mod future_runtime_continue_waker;
mod future_runtime_create_error;
mod future_runtime_future;
mod future_runtime_handler;
mod future_runtime_no_operation_yielder;
mod future_runtime_queue;
mod future_runtime_reference;
mod future_runtime_spawn_error;
mod future_runtime_task;
mod future_runtime_task_reference;
mod future_runtime_waker;
mod future_runtime_yielder;

pub use future_runtime::*;
pub use future_runtime_actor_address::*;
pub use future_runtime_actor_handler::*;
pub use future_runtime_allocator::*;
pub use future_runtime_box_future::*;
pub use future_runtime_concurrent_queue::*;
pub use future_runtime_continue_waker::*;
pub use future_runtime_create_error::*;
pub use future_runtime_future::*;
pub use future_runtime_handler::*;
pub use future_runtime_no_operation_yielder::*;
pub use future_runtime_queue::*;
pub use future_runtime_reference::*;
pub use future_runtime_spawn_error::*;
pub use future_runtime_task::*;
pub use future_runtime_task_reference::*;
pub use future_runtime_waker::*;
pub use future_runtime_yielder::*;
