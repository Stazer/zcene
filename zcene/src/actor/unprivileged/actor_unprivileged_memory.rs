use crate::actor::unprivileged::ActorUnprivilegedEnvironment;
use core::mem::{align_of, size_of};
use core::ptr::NonNull;
use zcene_core::actor::Actor;

////////////////////////////////////////////////////////////////////////////////////////////////////

const FRAME_SIZE: usize = 4096;

pub struct ActorUnprivilegedMemory<AI>
where
    AI: Actor<ActorUnprivilegedEnvironment>,
{
    actor: NonNull<AI>,
    message: NonNull<AI::Message>,
    stack_top: u64,
    poisoned: bool,
}

unsafe impl<AI> Send for ActorUnprivilegedMemory<AI> where AI: Actor<ActorUnprivilegedEnvironment> {}
unsafe impl<AI> Sync for ActorUnprivilegedMemory<AI> where AI: Actor<ActorUnprivilegedEnvironment> {}

impl<AI> ActorUnprivilegedMemory<AI>
where
    AI: Actor<ActorUnprivilegedEnvironment>,
{
    pub const STACK_SIZE: usize = 32 * 1024;

    const MESSAGE_OFFSET: usize = size_of::<AI>().next_multiple_of(align_of::<AI::Message>());

    const DATA_SIZE: usize =
        (Self::MESSAGE_OFFSET + size_of::<AI::Message>()).next_multiple_of(FRAME_SIZE);

    pub const SIZE: usize = Self::DATA_SIZE + Self::STACK_SIZE;

    pub fn new(memory: &'static mut [u8], actor: AI) -> Self {
        assert!(memory.len() >= Self::SIZE);
        assert!(memory.as_ptr() as usize % FRAME_SIZE == 0);
        assert!(align_of::<AI>() <= FRAME_SIZE && align_of::<AI::Message>() <= FRAME_SIZE);

        let base = memory.as_mut_ptr();

        let (actor_pointer, message_pointer) = unsafe {
            let actor_pointer = base as *mut AI;
            actor_pointer.write(actor);

            (
                NonNull::new_unchecked(actor_pointer),
                NonNull::new_unchecked(base.add(Self::MESSAGE_OFFSET) as *mut AI::Message),
            )
        };

        Self {
            actor: actor_pointer,
            message: message_pointer,
            stack_top: base as u64 + Self::SIZE as u64,
            poisoned: false,
        }
    }

    pub fn poison(&mut self) {
        self.poisoned = true;
    }

    pub fn actor(&mut self) -> *mut AI {
        self.actor.as_ptr()
    }

    pub fn stack_top(&self) -> u64 {
        self.stack_top
    }

    pub fn store_message(&mut self, message: AI::Message) -> *const AI::Message {
        unsafe { self.message.as_ptr().write(message) };
        self.message.as_ptr()
    }

    pub unsafe fn drop_message(&mut self) {
        unsafe { self.message.as_ptr().drop_in_place() };
    }
}

impl<AI> Drop for ActorUnprivilegedMemory<AI>
where
    AI: Actor<ActorUnprivilegedEnvironment>,
{
    fn drop(&mut self) {

        if !self.poisoned {
            unsafe { self.actor.as_ptr().drop_in_place() };
        }
    }
}
