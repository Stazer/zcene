use crate::actor::{
    ActorDiscovery, ActorEnvironment, ActorEnvironmentAllocator, ActorRecipient, ActorMessage,
    ActorWeakRecipient,
};
use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use core::any::{Any, TypeId};
use core::fmt::{self, Debug, Formatter};
use spin::Mutex as SpinMutex;

////////////////////////////////////////////////////////////////////////////////////////////////////

pub struct ActorSpinDiscovery<E>
where
    E: ActorEnvironment + ActorEnvironmentAllocator,
{
    allocator: E::Allocator,
    entries: SpinMutex<BTreeMap<TypeId, Box<dyn Any + Send + Sync, E::Allocator>, E::Allocator>>,
}

impl<E> Debug for ActorSpinDiscovery<E>
where
    E: ActorEnvironment + ActorEnvironmentAllocator,
{
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        formatter.debug_struct("ActorSpinDiscovery").finish()
    }
}

impl<E> ActorSpinDiscovery<E>
where
    E: ActorEnvironment + ActorEnvironmentAllocator,
{
    pub fn new_in(allocator: E::Allocator) -> Self {
        Self {
            entries: SpinMutex::new(BTreeMap::new_in(allocator.clone())),
            allocator,
        }
    }
}

impl<E> ActorDiscovery<E> for ActorSpinDiscovery<E>
where
    E: ActorEnvironment + ActorEnvironmentAllocator,
{
    fn register<M>(&self, recipient: ActorRecipient<M, E>)
    where
        M: ActorMessage,
    {
        let weak_recipient = recipient.downgrade();
        let mut entries = self.entries.lock();

        if let Some(entry) = entries.get_mut(&TypeId::of::<M>()) {
            entry
                .downcast_mut::<Vec<ActorWeakRecipient<M, E>, E::Allocator>>()
                .expect("actor discovery entry type mismatch")
                .push(weak_recipient);

            return;
        }

        let mut weak_recipients = Vec::new_in(self.allocator.clone());
        weak_recipients.push(weak_recipient);

        entries.insert(
            TypeId::of::<M>(),
            Box::new_in(weak_recipients, self.allocator.clone()),
        );
    }

    fn lookup<M>(&self) -> Vec<ActorRecipient<M, E>, E::Allocator>
    where
        M: ActorMessage,
    {
        let mut entries = self.entries.lock();

        let Some(entry) = entries.get_mut(&TypeId::of::<M>()) else {
            return Vec::new_in(self.allocator.clone());
        };

        let weak_recipients = entry
            .downcast_mut::<Vec<ActorWeakRecipient<M, E>, E::Allocator>>()
            .expect("actor discovery entry type mismatch");

        let mut recipients = Vec::new_in(self.allocator.clone());
        let mut index = 0;

        while index < weak_recipients.len() {
            match weak_recipients[index].upgrade() {
                Some(recipient) => {
                    recipients.push(recipient);
                    index += 1;
                }
                None => {
                    weak_recipients.swap_remove(index);
                }
            }
        }

        if weak_recipients.is_empty() {
            entries.remove(&TypeId::of::<M>());
        }

        recipients
    }
}
