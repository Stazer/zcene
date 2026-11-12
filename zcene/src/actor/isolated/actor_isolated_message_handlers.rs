use crate::actor::root::ActorRootEnvironment;
use crate::actor::isolated::ActorIsolatedMessage;
use crate::actor::isolated::{
    ActorIsolatedEnvironmentUnprivilegedPerspective, ActorIsolatedRecipient,
};
use crate::actor::unprivileged::{ActorUnprivilegedMessageHandler, ActorUnprivilegedMessageHandlers};
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
    ) -> Result<ActorIsolatedRecipient<M, ActorIsolatedEnvironmentUnprivilegedPerspective>, AllocError>
    where
        M: ActorIsolatedMessage,
    {
        Ok(ActorIsolatedRecipient::new(self.handlers.grant(recipient)?.index()))
    }

    pub fn grant_address<A>(
        &mut self,
        address: &ActorMessageChannelAddress<A, ActorRootEnvironment>,
    ) -> Result<
        ActorIsolatedRecipient<A::Message, ActorIsolatedEnvironmentUnprivilegedPerspective>,
        AllocError,
    >
    where
        A: Actor<ActorRootEnvironment>,
        A::Message: ActorIsolatedMessage,
    {
        Ok(ActorIsolatedRecipient::new(
            self.handlers.grant_address(address)?.index(),
        ))
    }
}
