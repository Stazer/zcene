use crate::kernel::actor::KernelActorHandler;
use zcene_core::actor::ActorAddressReference;

////////////////////////////////////////////////////////////////////////////////////////////////////

pub type KernelActorAddressReference<A> = ActorAddressReference<A, KernelActorHandler>;
