use crate::actor::root::ActorRootEnvironment;
use crate::actor::isolated::ActorIsolatedMessage;
use crate::actor::unprivileged::{
    ActorUnprivilegedEnvironmentUnprivilegedPerspective, ActorUnprivilegedMessageHandler,
    ActorUnprivilegedMessageHandlers, ActorUnprivilegedRecipient,
};
use crate::common::memory::allocator::ErasedAllocator;
use core::alloc::AllocError;
use zcene_core::actor::{Actor, ActorMessageChannelAddress, ActorRecipient};

////////////////////////////////////////////////////////////////////////////////////////////////////

pub struct ActorIsolatedMessageHandlers {
    handlers: ActorUnprivilegedMessageHandlers,
}

impl ActorIsolatedMessageHandlers {
    pub fn new(allocator: ErasedAllocator) -> Self {
        Self {
            handlers: ActorUnprivilegedMessageHandlers::new(allocator),
        }
    }

    pub(crate) fn get(
        &self,
        index: usize,
    ) -> Option<&dyn ActorUnprivilegedMessageHandler<ActorRootEnvironment>> {
        self.handlers.get(index)
    }

    pub fn grant<M>(
        &mut self,
        recipient: ActorRecipient<M, ActorRootEnvironment>,
    ) -> Result<ActorUnprivilegedRecipient<M, ActorUnprivilegedEnvironmentUnprivilegedPerspective>, AllocError>
    where
        M: ActorIsolatedMessage,
    {
        self.handlers.grant(recipient)
    }

    pub fn grant_address<A>(
        &mut self,
        address: &ActorMessageChannelAddress<A, ActorRootEnvironment>,
    ) -> Result<
        ActorUnprivilegedRecipient<A::Message, ActorUnprivilegedEnvironmentUnprivilegedPerspective>,
        AllocError,
    >
    where
        A: Actor<ActorRootEnvironment>,
        A::Message: ActorIsolatedMessage,
    {
        self.handlers.grant_address(address)
    }
}
