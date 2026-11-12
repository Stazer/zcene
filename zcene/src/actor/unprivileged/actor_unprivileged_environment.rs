use crate::actor::unprivileged::{
    ActorUnprivilegedEnvironmentPerspective, ActorUnprivilegedEnvironmentPrivilegedPerspective,
    ActorUnprivilegedEnvironmentUnprivilegedPerspective, ActorUnprivilegedRecipient,
    ActorUnprivilegedAddress, ActorUnprivilegedExecutor, ActorUnprivilegedImage, ActorUnprivilegedMessage,
    ActorUnprivilegedMessageHandlers,
};
use crate::actor::root::ActorRootEnvironment;
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

impl<A> ActorEnvironmentAllocator
    for ActorUnprivilegedEnvironment<ActorUnprivilegedEnvironmentPrivilegedPerspective, A>
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

    /// The kernel only knows an image, not the actor type inside it, so the caller names the
    /// message type `M` the image accepts and gets a recipient for it. Whether `M` really is what
    /// the image expects is not checked.
    ///
    /// The image is a flat binary that is copied to the start of a fresh address space and entered
    /// at `entry_offset` for every `ActorUnprivilegedPhase`.
    pub fn spawn<M>(
        self: &ActorEnvironmentReference<Self>,
        image: ActorUnprivilegedImage,
        deadline_in_milliseconds: Option<NonZero<usize>>,
        message_handlers: ActorUnprivilegedMessageHandlers,
    ) -> Result<
        ActorUnprivilegedRecipient<M, ActorUnprivilegedEnvironmentPrivilegedPerspective>,
        ActorSpawnError,
    >
    where
        M: ActorUnprivilegedMessage,
    {
        let root = ActorRootEnvironment::get();
        let allocator = self.allocator.clone();

        let (sender, receiver) =
            ActorMessageChannel::<ActorMessageEnvelope<M, ActorRootEnvironment>>::new_unbounded_in(
                allocator.clone(),
            );

        let recipient: ActorRecipient<M, ActorRootEnvironment> =
            ActorRecipient::from_channel(sender, allocator)?;

        root.discovery().register(recipient.clone());

        root.future_runtime().spawn(ActorUnprivilegedExecutor::<M>::new(
            root.clone(),
            image,
            receiver,
            recipient.clone(),
            deadline_in_milliseconds,
            message_handlers,
        )?)?;

        Ok(ActorUnprivilegedRecipient::from_recipient(recipient))
    }
}
