use core::cell::UnsafeCell;
use core::mem::MaybeUninit;

////////////////////////////////////////////////////////////////////////////////////////////////////

/// Where an isolated program keeps its actor between the phases.
pub struct ActorIsolatedCell<A> {
    actor: UnsafeCell<MaybeUninit<A>>,
}

unsafe impl<A> Sync for ActorIsolatedCell<A> {}

impl<A> ActorIsolatedCell<A> {
    pub const fn new() -> Self {
        Self {
            actor: UnsafeCell::new(MaybeUninit::uninit()),
        }
    }

    /// # Safety
    ///
    /// Must not be called while an actor is stored, and not concurrently.
    pub unsafe fn store(&self, actor: A) -> &mut A {
        unsafe { (*self.actor.get()).write(actor) }
    }

    /// # Safety
    ///
    /// An actor must be stored, and the returned reference must be the only one.
    pub unsafe fn get(&self) -> &mut A {
        unsafe { (*self.actor.get()).assume_init_mut() }
    }

    /// # Safety
    ///
    /// An actor must be stored, and it must not be used afterwards.
    pub unsafe fn drop(&self) {
        unsafe { (*self.actor.get()).assume_init_drop() }
    }
}
