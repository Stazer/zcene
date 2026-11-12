use crate::actor::unprivileged::{
    ActorUnprivilegedEnvironmentUnprivilegedPerspective, ActorUnprivilegedMessage, ActorUnprivilegedRecipient,
};
use crate::actor::root::ActorRootEnvironment;
use crate::common::memory::allocator::ErasedAllocator;
use alloc::boxed::Box;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::alloc::{AllocError, Layout};
use core::ptr::NonNull;
use zcene_core::actor::{
    Actor, ActorAddressExt, ActorBoxFuture, ActorEnvironment, ActorEnvironmentAllocator,
    ActorMessage, ActorMessageChannelAddress, ActorMessageSender, ActorRecipient, ActorSendError,
};

////////////////////////////////////////////////////////////////////////////////////////////////////

/// What an unprivileged program can reach: a recipient of the kernel that the program sends raw
/// message bytes to.
pub trait ActorUnprivilegedMessageHandler<E>: Send + Sync + 'static
where
    E: ActorEnvironment + ActorEnvironmentAllocator,
{
    fn send(
        &self,
        allocator: &E::Allocator,
        message: *const (),
    ) -> ActorBoxFuture<'static, Result<(), ActorSendError>, E>;

    fn message_layout(&self) -> Layout;
}

impl<M, E> ActorUnprivilegedMessageHandler<E> for ActorRecipient<M, E>
where
    M: ActorMessage,
    E: ActorEnvironment + ActorEnvironmentAllocator,
{
    fn message_layout(&self) -> Layout {
        Layout::new::<M>()
    }

    fn send(
        &self,
        allocator: &E::Allocator,
        message: *const (),
    ) -> ActorBoxFuture<'static, Result<(), ActorSendError>, E> {
        let recipient = self.clone();

        let pointer = if Layout::new::<M>().size() == 0 {
            NonNull::<M>::dangling().as_ptr().cast_const()
        } else {
            message.cast::<M>()
        };

        let message = unsafe { pointer.as_ref().unwrap() }.clone();

        Box::pin_in(
            async move { ActorMessageSender::send(&recipient, message).await },
            allocator.clone(),
        )
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////

/// The recipients the kernel grants to one program. The program addresses them by index.
pub struct ActorUnprivilegedMessageHandlers {
    allocator: ErasedAllocator,
    handlers: Vec<
        Box<dyn ActorUnprivilegedMessageHandler<ActorRootEnvironment>, ErasedAllocator>,
        ErasedAllocator,
    >,
}

impl ActorUnprivilegedMessageHandlers {
    pub fn new(allocator: ErasedAllocator) -> Self {
        Self {
            handlers: Vec::new_in(allocator.clone()),
            allocator,
        }
    }

    pub(crate) fn get(
        &self,
        index: usize,
    ) -> Option<&dyn ActorUnprivilegedMessageHandler<ActorRootEnvironment>> {
        self.handlers.get(index).map(|handler| &**handler)
    }

    pub fn grant<M>(
        &mut self,
        recipient: ActorRecipient<M, ActorRootEnvironment>,
    ) -> Result<ActorUnprivilegedRecipient<M, ActorUnprivilegedEnvironmentUnprivilegedPerspective>, AllocError>
    where
        M: ActorUnprivilegedMessage,
    {
        let index = self.handlers.len();

        self.handlers.try_reserve(1).map_err(|_| AllocError)?;
        self.handlers
            .push(Box::try_new_in(recipient, self.allocator.clone())?);

        Ok(ActorUnprivilegedRecipient::new(index))
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
        A::Message: ActorUnprivilegedMessage,
    {
        let address = Arc::try_new_in(address.clone(), self.allocator.clone())?;

        self.grant(address.recipient()?)
    }
}
