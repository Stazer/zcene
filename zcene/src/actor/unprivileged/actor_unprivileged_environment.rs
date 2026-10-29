use crate::actor::root::ActorRootEnvironment;
use crate::actor::unprivileged::{
    ActorUnprivilegedAddress, ActorUnprivilegedExecutor, ActorUnprivilegedMessageHandler,
    ActorUnprivilegedRootProxy,
};
use crate::common::memory::allocator::ErasedAllocator;
use alloc::boxed::Box;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::num::NonZero;
use zcene_core::actor::{
    Actor, ActorAllocator, ActorCommonHandleContext, ActorEnvironment, ActorEnvironmentAllocator,
    ActorEnvironmentReference, ActorRecipient, ActorMessage, ActorMessageChannel,
    ActorMessageChannelAddress, ActorMessageEnvelope, ActorSpawnError, ActorAddressExt,
    ActorDiscovery,
};

////////////////////////////////////////////////////////////////////////////////////////////////////

pub type ActorUnprivilegedMessageHandlers = Vec<
    Box<dyn ActorUnprivilegedMessageHandler<ActorRootEnvironment> + Send + Sync, ErasedAllocator>,
    ErasedAllocator,
>;

/// Address of an unprivileged actor as seen from the root environment.
pub type ActorUnprivilegedRootAddress<A> =
    ActorMessageChannelAddress<ActorUnprivilegedRootProxy<A>, ActorRootEnvironment>;

////////////////////////////////////////////////////////////////////////////////////////////////////

/// The environment unprivileged actors are spawned into. It is part of every
/// `ActorRootEnvironment` (see `ActorRootEnvironment::unprivileged_environment`).
pub struct ActorUnprivilegedEnvironment<A = ErasedAllocator>
where
    A: ActorAllocator,
{
    allocator: A,
}

impl ActorEnvironment for ActorUnprivilegedEnvironment {
    type Address<A>
        = ActorUnprivilegedAddress<A>
    where
        A: Actor<Self>;

    type CreateContext<'a> = ();

    type HandleContext<'a, M>
        = ActorCommonHandleContext<M>
    where
        M: ActorMessage;

    type DestroyContext<'a> = ();
}

impl<A> ActorEnvironmentAllocator for ActorUnprivilegedEnvironment<A>
where
    A: ActorAllocator,
{
    type Allocator = A;

    fn allocator(&self) -> &Self::Allocator {
        &self.allocator
    }
}

impl<A> ActorUnprivilegedEnvironment<A>
where
    A: ActorAllocator,
{
    pub(crate) fn new(allocator: A) -> ActorEnvironmentReference<Self> {
        Arc::new_in(Self { allocator: allocator.clone() }, allocator)
    }
}

impl ActorUnprivilegedEnvironment {

    /// Spawns `actor` unprivileged (ring 3 on x86_64). `deadline_in_milliseconds` bounds how long
    /// it may run before being preempted; `message_handlers` are the root recipients it can reach
    /// via `ActorUnprivilegedAddress::new(index)`. The actor is reachable from the root through
    /// the returned address and through discovery.
    pub fn spawn<A>(
        self: &ActorEnvironmentReference<Self>,
        actor: A,
        deadline_in_milliseconds: Option<NonZero<usize>>,
        message_handlers: ActorUnprivilegedMessageHandlers,
    ) -> Result<ActorUnprivilegedRootAddress<A>, ActorSpawnError>
    where
        A: Actor<ActorUnprivilegedEnvironment>,
    {
        let root = ActorRootEnvironment::get();
        let allocator = self.allocator.clone();

        let (sender, receiver) =
            ActorMessageChannel::<ActorMessageEnvelope<A::Message, ActorRootEnvironment>>::new_unbounded_in(
                allocator.clone(),
            );

        let address = ActorUnprivilegedRootAddress::<A>::new(sender, allocator.clone());
        let address_reference = Arc::new_in(address.clone(), allocator);
        let recipient: ActorRecipient<A::Message, ActorRootEnvironment> = address_reference.recipient()?;

        root.discovery().register(recipient.clone());

        root.future_runtime().spawn(ActorUnprivilegedExecutor::<A>::new(
            root.clone(),
            actor,
            receiver,
            recipient,
            deadline_in_milliseconds,
            message_handlers,
        )?)?;

        Ok(address)
    }
}
