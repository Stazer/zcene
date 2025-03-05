mod actor_isolation_address;
mod actor_isolation_environment;
mod actor_isolation_executor;
mod actor_isolation_executor_context;
mod actor_isolation_executor_create_state;
mod actor_isolation_executor_deadline_preemption_context;
mod actor_isolation_executor_deadline_preemption_event;
mod actor_isolation_executor_destroy_state;
mod actor_isolation_executor_event;
mod actor_isolation_executor_handle_state;
mod actor_isolation_executor_receive_state;
mod actor_isolation_executor_result;
mod actor_isolation_executor_state;
mod actor_isolation_executor_state_handler;
mod actor_isolation_executor_system_call_context;
mod actor_isolation_executor_system_call_event;
mod actor_isolation_executor_system_call_type;
mod actor_isolation_spawn_specification;

pub use actor_isolation_address::*;
pub use actor_isolation_environment::*;
pub use actor_isolation_executor::*;
pub use actor_isolation_executor_context::*;
pub use actor_isolation_executor_create_state::*;
pub use actor_isolation_executor_deadline_preemption_context::*;
pub use actor_isolation_executor_deadline_preemption_event::*;
pub use actor_isolation_executor_destroy_state::*;
pub use actor_isolation_executor_event::*;
pub use actor_isolation_executor_handle_state::*;
pub use actor_isolation_executor_receive_state::*;
pub use actor_isolation_executor_result::*;
pub use actor_isolation_executor_state::*;
pub use actor_isolation_executor_state_handler::*;
pub use actor_isolation_executor_system_call_context::*;
pub use actor_isolation_executor_system_call_event::*;
pub use actor_isolation_executor_system_call_type::*;
pub use actor_isolation_spawn_specification::*;

use crate::actor::ActorRootEnvironment;
use alloc::boxed::Box;
use core::fmt::Debug;
use zcene_core::actor::{
    Actor, ActorBoxFuture, ActorCommonBounds, ActorEnvironment, ActorEnvironmentAllocator,
    ActorMessage, ActorMessageChannelAddress, ActorMessageChannelSender, ActorMessageSender,
};
use zcene_core::future::runtime::FutureRuntimeHandler;

pub trait ActorIsolationMessageHandler<E>: ActorCommonBounds
where
    E: ActorEnvironment + ActorEnvironmentAllocator,
{
    fn send(&self, allocator: &E::Allocator, message: *const ()) -> ActorBoxFuture<'static, (), E>;
}

impl<A, E> ActorIsolationMessageHandler<E> for ActorMessageChannelAddress<A, E>
where
    A: Actor<E>,
    A::Message: Debug,
    E: ActorEnvironment + ActorEnvironmentAllocator,
{
    fn send(&self, allocator: &E::Allocator, message: *const ()) -> ActorBoxFuture<'static, (), E> {
        let sender = self.clone();

        // TODO: check address and length!
        let message = unsafe { message.cast::<A::Message>().as_ref().unwrap() }.clone();

        Box::pin_in(
            async move {
                <Self as ActorMessageSender<_>>::send(&sender, message).await;
            },
            allocator.clone(),
        )
    }
}
