use ztd::{Constructor, Method};
use zcene_kernel::memory::address::VirtualMemoryAddress;
use crate::actor::ActorThreadType;

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Constructor, Debug, Method)]
#[Method(all)]
pub struct ActorThread {
    r#type: ActorThreadType,
    stack_pointer: Option<VirtualMemoryAddress>,
}
