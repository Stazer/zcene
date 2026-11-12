use crate::actor::unprivileged::{
    ActorUnprivilegedEnvironment, ActorUnprivilegedMemory, ActorUnprivilegedProgram,
    ActorUnprivilegedProgramEntry,
};
use crate::platform::{ActivePlatform, Platform};
use core::alloc::{AllocError, Layout};
use core::future::Future;
use core::pin::pin;
use core::task::{Context, Poll, Waker};
use crate::actor::ActorIsolatedPhase;
use zcene_core::actor::{Actor, ActorCommonHandleContext};

////////////////////////////////////////////////////////////////////////////////////////////////////

pub struct ActorUnprivilegedActorProgram<AI>
where
    AI: Actor<ActorUnprivilegedEnvironment>,
{
    memory: ActorUnprivilegedMemory<AI>,
}

impl<AI> ActorUnprivilegedActorProgram<AI>
where
    AI: Actor<ActorUnprivilegedEnvironment>,
{
    pub fn new(platform: &ActivePlatform, actor: AI) -> Result<Self, AllocError> {
        let memory = platform
            .allocate_unprivileged_memory(ActorUnprivilegedMemory::<AI>::SIZE)
            .ok_or(AllocError)?;

        Ok(Self {
            memory: ActorUnprivilegedMemory::new(memory, actor),
        })
    }

    extern "C" fn create_entry(actor: *mut AI, _message: *const ()) -> ! {
        let actor = unsafe { &mut *actor };
        let _ = Self::poll_to_completion(actor.create(()));
        ActivePlatform::complete_unprivileged_execution()
    }

    extern "C" fn handle_entry(actor: *mut AI, message: *const ()) -> ! {
        let actor = unsafe { &mut *actor };
        let message = unsafe { &*(message as *const AI::Message) };
        let _ = Self::poll_to_completion(actor.handle(ActorCommonHandleContext::new(message.clone())));
        ActivePlatform::complete_unprivileged_execution()
    }

    extern "C" fn destroy_entry(actor: *mut AI, _message: *const ()) -> ! {
        let actor = unsafe { &mut *actor };
        let _ = Self::poll_to_completion(actor.destroy(()));
        ActivePlatform::complete_unprivileged_execution()
    }

    fn poll_to_completion<F: Future>(future: F) -> F::Output {
        let mut context = Context::from_waker(Waker::noop());
        match pin!(future).poll(&mut context) {
            Poll::Ready(result) => result,
            Poll::Pending => todo!("unprivileged actor futures must not return Pending"),
        }
    }
}

impl<AI> ActorUnprivilegedProgram for ActorUnprivilegedActorProgram<AI>
where
    AI: Actor<ActorUnprivilegedEnvironment>,
{
    type Message = AI::Message;

    fn address_space(&self) -> Option<&<ActivePlatform as Platform>::UnprivilegedAddressSpace> {
        None
    }

    fn stack_top(&self) -> u64 {
        self.memory.stack_top()
    }

    fn entry(&mut self, phase: ActorIsolatedPhase) -> ActorUnprivilegedProgramEntry {
        let entry: extern "C" fn(*mut AI, *const ()) -> ! = match phase {
            ActorIsolatedPhase::Create => Self::create_entry,
            ActorIsolatedPhase::Handle => Self::handle_entry,
            ActorIsolatedPhase::Destroy => Self::destroy_entry,
        };

        ActorUnprivilegedProgramEntry {
            entry: unsafe {
                core::mem::transmute::<
                    extern "C" fn(*mut AI, *const ()) -> !,
                    extern "C" fn(*mut (), *const ()) -> !,
                >(entry)
            },
            argument: self.memory.actor() as *mut (),
        }
    }

    fn store_message(&mut self, message: Self::Message) -> u64 {
        self.memory.store_message(message) as usize as u64
    }

    fn drop_message(&mut self) {
        unsafe { self.memory.drop_message() };
    }

    fn contains(&self, address: u64, layout: Layout) -> bool {
        self.memory.contains(address as usize, layout)
    }

    fn kernel_address(&self, address: u64) -> usize {
        address as usize
    }

    fn poison(&mut self) {
        self.memory.poison();
    }
}
