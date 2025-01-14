use crate::actor::{ActorHandler, ActorSystem};
use alloc::sync::Arc;

////////////////////////////////////////////////////////////////////////////////////////////////////

pub type ActorSystemReference<H> = Arc<ActorSystem<H>, <H as ActorHandler>::Allocator>;
