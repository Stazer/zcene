use crate::actor::ActorSpawnSpecificationType;
use core::marker::PhantomData;
use zcene_core::actor::{Actor, ActorHandler};
use ztd::{Constructor, Inner, Method};

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Constructor, Inner, Method)]
#[Method(accessors)]
pub struct ActorSpawnSpecification<A, H>
where
    A: Actor<H>,
    H: ActorHandler,
{
    actor: A,
    r#type: ActorSpawnSpecificationType,
    #[Constructor(default)]
    marker: PhantomData<H>,
}
