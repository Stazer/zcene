use crate::actor::ActorThreadType;
use zcene_kernel::memory::address::VirtualMemoryAddress;
use ztd::{Constructor, Method};

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Constructor, Debug, Method)]
#[Method(all)]
pub struct ActorThread {
    r#type: ActorThreadType,
    stack_pointer: Option<VirtualMemoryAddress>,
}
