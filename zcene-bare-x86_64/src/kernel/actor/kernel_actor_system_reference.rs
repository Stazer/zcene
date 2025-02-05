use crate::kernel::actor::KernelActorHandler;
use zcene_core::actor::ActorSystemReference;

////////////////////////////////////////////////////////////////////////////////////////////////////

pub type KernelActorSystemReference = ActorSystemReference<KernelActorHandler>;
