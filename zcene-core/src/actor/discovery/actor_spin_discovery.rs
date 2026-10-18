use crate::actor::{
    ActorDiscovery, ActorEnvironment, ActorEnvironmentAllocator, ActorMailbox, ActorMessage,
    ActorWeakMailbox,
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
    fn register<M>(&self, mailbox: ActorMailbox<M, E>)
    where
        M: ActorMessage,
    {
        let weak_mailbox = mailbox.downgrade();
        let mut entries = self.entries.lock();

        if let Some(entry) = entries.get_mut(&TypeId::of::<M>()) {
            entry
                .downcast_mut::<Vec<ActorWeakMailbox<M, E>, E::Allocator>>()
                .expect("actor discovery entry type mismatch")
                .push(weak_mailbox);

            return;
        }

        let mut weak_mailboxes = Vec::new_in(self.allocator.clone());
        weak_mailboxes.push(weak_mailbox);

        entries.insert(
            TypeId::of::<M>(),
            Box::new_in(weak_mailboxes, self.allocator.clone()),
        );
    }

    fn lookup<M>(&self) -> Vec<ActorMailbox<M, E>, E::Allocator>
    where
        M: ActorMessage,
    {
        let mut entries = self.entries.lock();

        let Some(entry) = entries.get_mut(&TypeId::of::<M>()) else {
            return Vec::new_in(self.allocator.clone());
        };

        let weak_mailboxes = entry
            .downcast_mut::<Vec<ActorWeakMailbox<M, E>, E::Allocator>>()
            .expect("actor discovery entry type mismatch");

        let mut mailboxes = Vec::new_in(self.allocator.clone());
        let mut index = 0;

        while index < weak_mailboxes.len() {
            match weak_mailboxes[index].upgrade() {
                Some(mailbox) => {
                    mailboxes.push(mailbox);
                    index += 1;
                }
                None => {
                    weak_mailboxes.swap_remove(index);
                }
            }
        }

        if weak_mailboxes.is_empty() {
            entries.remove(&TypeId::of::<M>());
        }

        mailboxes
    }
}
