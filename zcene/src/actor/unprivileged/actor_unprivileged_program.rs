use crate::platform::{ActivePlatform, Platform};
use core::alloc::Layout;
use crate::actor::ActorIsolatedPhase;
use zcene_core::actor::ActorMessage;

////////////////////////////////////////////////////////////////////////////////////////////////////

pub struct ActorUnprivilegedProgramEntry {
    pub entry: extern "C" fn(*mut (), *const ()) -> !,
    pub argument: *mut (),
}

pub trait ActorUnprivilegedProgram: Send + 'static {
    type Message: ActorMessage;

    fn address_space(&self) -> Option<&<ActivePlatform as Platform>::UnprivilegedAddressSpace>;

    fn stack_top(&self) -> u64;

    fn entry(&mut self, phase: ActorIsolatedPhase) -> ActorUnprivilegedProgramEntry;

    fn store_message(&mut self, message: Self::Message) -> u64;

    fn drop_message(&mut self);

    fn contains(&self, address: u64, layout: Layout) -> bool;

    fn kernel_address(&self, address: u64) -> usize;

    fn poison(&mut self);
}
