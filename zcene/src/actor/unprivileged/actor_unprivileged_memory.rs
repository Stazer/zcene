use crate::actor::unprivileged::ActorUnprivilegedEnvironment;
use core::mem::{align_of, size_of};
use core::ptr::NonNull;
use zcene_core::actor::Actor;

////////////////////////////////////////////////////////////////////////////////////////////////////

const FRAME_SIZE: usize = 4096;

/// The memory an unprivileged actor owns: the actor itself, a slot for the message currently being
/// handled, and the stack it runs on, all in one user-accessible region. No allocator is involved,
/// everything sits at a fixed offset.
///
/// ```text
/// base                                       base + size
/// | actor | message slot | (padding) | stack ->  |
///                                       grows down
/// ```
///
/// The stack is last, so an overflow runs into this actor's own data rather than a neighbour's.
pub struct ActorUnprivilegedMemory<AI>
where
    AI: Actor<ActorUnprivilegedEnvironment>,
{
    actor: NonNull<AI>,
    message: NonNull<AI::Message>,
    stack_top: u64,
    poisoned: bool,
}

// SAFETY: the region is owned exclusively by this value, and `AI` and its message are `Send`.
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

    /// Bytes of user memory needed for one actor.
    pub const SIZE: usize = Self::DATA_SIZE + Self::STACK_SIZE;

    /// Moves `actor` into `memory`, which must be `SIZE` long and frame-aligned.
    pub fn new(memory: &'static mut [u8], actor: AI) -> Self {
        assert!(memory.len() >= Self::SIZE);
        assert!(memory.as_ptr() as usize % FRAME_SIZE == 0);
        assert!(align_of::<AI>() <= FRAME_SIZE && align_of::<AI::Message>() <= FRAME_SIZE);

        let base = memory.as_mut_ptr();

        // SAFETY: both offsets are inside `memory` and aligned, see the assertions above.
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

    /// Marks the actor as corrupt, e.g. after it faulted: it is then never touched by the kernel
    /// again, not even to drop it.
    pub fn poison(&mut self) {
        self.poisoned = true;
    }

    pub fn actor(&mut self) -> *mut AI {
        self.actor.as_ptr()
    }

    pub fn stack_top(&self) -> u64 {
        self.stack_top
    }

    /// Moves `message` into the message slot. Must be paired with `drop_message`.
    pub fn store_message(&mut self, message: AI::Message) -> *const AI::Message {
        unsafe { self.message.as_ptr().write(message) };
        self.message.as_ptr()
    }

    /// # Safety
    ///
    /// The slot must hold a message from `store_message`.
    pub unsafe fn drop_message(&mut self) {
        unsafe { self.message.as_ptr().drop_in_place() };
    }
}

impl<AI> Drop for ActorUnprivilegedMemory<AI>
where
    AI: Actor<ActorUnprivilegedEnvironment>,
{
    fn drop(&mut self) {
        // The memory itself is not returned to the frame allocator yet.
        if !self.poisoned {
            unsafe { self.actor.as_ptr().drop_in_place() };
        }
    }
}
