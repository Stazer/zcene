mod actor;
mod actor_allocator;
mod actor_box_future;
mod actor_common_bounds;
mod actor_future;
mod actor_handler;
mod actor_mailbox;
mod actor_mailbox_message_sender;
mod actor_system;
mod actor_system_reference;
mod address;
mod context;
mod error;
mod inline_actor;
mod message;
mod root_actor;

pub use actor::*;
pub use actor_allocator::*;
pub use actor_box_future::*;
pub use actor_common_bounds::*;
pub use actor_future::*;
pub use actor_handler::*;
pub use actor_mailbox::*;
pub use actor_mailbox_message_sender::*;
pub use actor_system::*;
pub use actor_system_reference::*;
pub use address::*;
pub use context::*;
pub use error::*;
pub use inline_actor::*;
pub use message::*;
pub use root_actor::*;

use ztd::Constructor;

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Constructor)]
pub struct ActorCommonHandleContext<M>
where
    M: ActorMessage,
{
    message: M,
}

impl<M> ActorContextMessageProvider<M> for ActorCommonHandleContext<M>
where
    M: ActorMessage,
{
    fn message(&self) -> &M {
        &self.message
    }
}
