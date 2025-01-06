#![no_std]
#![feature(trait_alias)]
#![feature(allocator_api)]
#![feature(arbitrary_self_types)]
#![feature(fn_traits)]
#![feature(unboxed_closures)]

////////////////////////////////////////////////////////////////////////////////////////////////////

extern crate alloc;

////////////////////////////////////////////////////////////////////////////////////////////////////

mod actor;
mod actor_address;
mod actor_address_reference;
mod actor_allocator;
mod actor_box_future;
mod actor_common_bounds;
mod actor_context_message_provider;
mod actor_create_error;
mod actor_destroy_error;
mod actor_enter_error;
mod actor_future;
mod actor_handle_error;
mod actor_handler;
mod actor_mailbox;
mod actor_message;
mod actor_message_sender;
mod actor_send_error;
mod actor_spawn_error;
mod actor_system;
mod actor_system_create_error;
mod actor_system_reference;
mod future_ext;

pub use actor::*;
pub use actor_address::*;
pub use actor_address_reference::*;
pub use actor_allocator::*;
pub use actor_box_future::*;
pub use actor_common_bounds::*;
pub use actor_context_message_provider::*;
pub use actor_create_error::*;
pub use actor_destroy_error::*;
pub use actor_enter_error::*;
pub use actor_future::*;
pub use actor_handle_error::*;
pub use actor_handler::*;
pub use actor_mailbox::*;
pub use actor_message::*;
pub use actor_message_sender::*;
pub use actor_send_error::*;
pub use actor_spawn_error::*;
pub use actor_system::*;
pub use actor_system_create_error::*;
pub use actor_system_reference::*;
pub use future_ext::*;
