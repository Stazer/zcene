#![no_std]
#![no_main]

////////////////////////////////////////////////////////////////////////////////////////////////////

use zcene::actor::{ActorIsolatedEnvironment, ActorIsolatedRecipient};
use zcene::core::actor::{Actor, ActorEnvironment, ActorHandleError, ActorMessageSender};
use zcene::define_isolated_actor;

////////////////////////////////////////////////////////////////////////////////////////////////////

pub struct DoublerActor {
    root: ActorIsolatedRecipient<u32>,
}

impl Actor<ActorIsolatedEnvironment> for DoublerActor {
    type Message = u32;

    async fn handle<'a>(
        &'a mut self,
        context: <ActorIsolatedEnvironment as ActorEnvironment>::HandleContext<'a, Self::Message>,
    ) -> Result<(), ActorHandleError> {
        let _ = self.root.send(2_000_000 + 2 * *context.message()).await;

        Ok(())
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////

define_isolated_actor!(
    DoublerActor,
    DoublerActor {
        root: ActorIsolatedRecipient::new(0),
    }
);
