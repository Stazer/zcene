use crate::actor::{ActorIsolationEnvironment, ActorRootEnvironment};
use alloc::boxed::Box;
use core::marker::PhantomData;
use zcene_core::actor::{
    Actor, ActorEnvironment, ActorMessageChannel, ActorMessageChannelReceiver,
};
use zcene_core::future::runtime::FutureRuntimeHandler;
use ztd::{Constructor, Inner};

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Constructor, Inner)]
pub struct ActorIsolationExecutorHandleState<AI, AR, H>
where
    AI: Actor<ActorIsolationEnvironment>,
    AR: Actor<ActorRootEnvironment<H>>,
    H: FutureRuntimeHandler,
{
    actor: Box<AI>,
    message: AR::Message,
    #[Constructor(default)]
    marker: PhantomData<(AR, H)>,
}
