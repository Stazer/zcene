use crate::kernel::actor::KernelActorThreadType;
use zcene_bare::memory::address::VirtualMemoryAddress;
use ztd::{Constructor, Method};

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Constructor, Debug, Method)]
#[Method(all)]
pub struct KernelActorThread {
    r#type: KernelActorThreadType,
    stack_pointer: Option<VirtualMemoryAddress>,
}
