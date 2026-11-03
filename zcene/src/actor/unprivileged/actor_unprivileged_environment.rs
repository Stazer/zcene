use crate::actor::root::ActorRootEnvironment;
use crate::actor::unprivileged::{
    ActorUnprivilegedAddress, ActorUnprivilegedExecutor, ActorUnprivilegedEnvironmentPrivilegedPerspective,
    ActorUnprivilegedMessageHandlers, ActorUnprivilegedEnvironmentPerspective, ActorUnprivilegedRecipient,
    ActorUnprivilegedEnvironmentUnprivilegedPerspective,
};
use crate::common::memory::allocator::ErasedAllocator;
use alloc::sync::Arc;
use core::marker::PhantomData;
use core::num::NonZero;
use zcene_core::actor::{
    Actor, ActorAllocator, ActorCommonHandleContext, ActorDiscovery, ActorEnvironment,
    ActorEnvironmentAllocator, ActorEnvironmentReference, ActorMessage, ActorMessageChannel,
    ActorMessageEnvelope, ActorRecipient, ActorSpawnError,
};

////////////////////////////////////////////////////////////////////////////////////////////////////

pub struct ActorUnprivilegedEnvironment<
    P = ActorUnprivilegedEnvironmentUnprivilegedPerspective,
    A = ErasedAllocator,
> where
    P: ActorUnprivilegedEnvironmentPerspective,
    A: ActorAllocator,
{
    allocator: A,
    marker: PhantomData<fn() -> P>,
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

impl<A> ActorEnvironmentAllocator for ActorUnprivilegedEnvironment<ActorUnprivilegedEnvironmentPrivilegedPerspective, A>
where
    A: ActorAllocator,
{
    type Allocator = A;

    fn allocator(&self) -> &Self::Allocator {
        &self.allocator
    }
}

impl<A> ActorUnprivilegedEnvironment<ActorUnprivilegedEnvironmentPrivilegedPerspective, A>
where
    A: ActorAllocator,
{
    pub(crate) fn new(allocator: A) -> ActorEnvironmentReference<Self> {
        Arc::new_in(
            Self {
                allocator: allocator.clone(),
                marker: PhantomData,
            },
            allocator,
        )
    }
}

impl ActorUnprivilegedEnvironment<ActorUnprivilegedEnvironmentPrivilegedPerspective, ErasedAllocator> {

    pub fn handlers(&self) -> ActorUnprivilegedMessageHandlers {
        ActorUnprivilegedMessageHandlers::new(self.allocator.clone())
    }

    pub fn spawn<A>(
        self: &ActorEnvironmentReference<Self>,
        actor: A,
        deadline_in_milliseconds: Option<NonZero<usize>>,
        message_handlers: ActorUnprivilegedMessageHandlers,
    ) -> Result<ActorUnprivilegedAddress<A, ActorUnprivilegedEnvironmentPrivilegedPerspective>, ActorSpawnError>
    where
        A: Actor<ActorUnprivilegedEnvironment>,
    {
        let root = ActorRootEnvironment::get();
        let allocator = self.allocator.clone();

        let (sender, receiver) =
            ActorMessageChannel::<ActorMessageEnvelope<A::Message, ActorRootEnvironment>>::new_unbounded_in(
                allocator.clone(),
            );

        let recipient: ActorRecipient<A::Message, ActorRootEnvironment> =
            ActorRecipient::from_channel(sender, allocator)?;

        root.discovery().register(recipient.clone());

        root.future_runtime().spawn(ActorUnprivilegedExecutor::<A>::new(
            root.clone(),
            actor,
            receiver,
            recipient.clone(),
            deadline_in_milliseconds,
            message_handlers,
        )?)?;

        Ok(ActorUnprivilegedAddress::new(
            ActorUnprivilegedRecipient::from_recipient(recipient),
        ))
    }
}
