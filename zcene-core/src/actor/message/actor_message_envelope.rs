use crate::actor::{ActorDyn, ActorEnvironment, ActorEnvironmentAllocator, ActorMessage};
use alloc::boxed::Box;

////////////////////////////////////////////////////////////////////////////////////////////////////

/// What actually flows through a channel-backed recipient: either a regular message for the
/// currently running actor, or a request to replace that actor with a different
/// implementation (same message type) without losing the recipient's identity or any
/// messages still queued behind it.
pub enum ActorMessageEnvelope<M, E>
where
    M: ActorMessage,
    E: ActorEnvironment + ActorEnvironmentAllocator,
{
    Message(M),
    Migrate(Box<dyn ActorDyn<E, Message = M>, E::Allocator>),
}
