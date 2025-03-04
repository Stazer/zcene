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
use zcene_core::actor::{
    Actor, ActorBoxFuture, ActorCommonBounds, ActorEnvironment, ActorEnvironmentAllocator,
    ActorMessage, ActorMessageChannelAddress, ActorMessageChannelSender,
};
use zcene_core::future::runtime::FutureRuntimeHandler;

pub trait ActorIsolationMessageHandler: Send + Sync/*: ActorCommonBounds*/
//where
    //E: ActorEnvironment + ActorEnvironmentAllocator,
{
    fn send(&self, address: usize) /*-> ActorBoxFuture<'static, (), E>*/;
}

/*impl<A, E> ActorIsolationMessageHandler<E> for ActorMessageChannelAddress<A, E>
where
    A: Actor<E>,
    E: ActorEnvironment + ActorEnvironmentAllocator,
{
    fn send(&self, allocator: &E::Allocator, address: usize) -> ActorBoxFuture<'static, (), E> {
        Box::pin_in(async move { todo!() }, allocator.clone())
    }
}*/

impl ActorIsolationMessageHandler for ()
//where
    //E: ActorEnvironment + ActorEnvironmentAllocator,
{
    fn send(&self, address: usize) /*-> ActorBoxFuture<'static, (), E>*/ {
        todo!()
        //Box::pin_in(async move { todo!() }, allocator.clone())
    }
}
