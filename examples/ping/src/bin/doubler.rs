#![no_std]
#![no_main]

////////////////////////////////////////////////////////////////////////////////////////////////////

use zcene::actor::{ActorUnprivilegedEnvironment, ActorUnprivilegedRecipient};
use zcene::core::actor::{Actor, ActorEnvironment, ActorHandleError, ActorMessageSender};
use zcene::define_unprivileged_actor;

////////////////////////////////////////////////////////////////////////////////////////////////////

pub struct DoublerActor {
    root: ActorUnprivilegedRecipient<u32>,
}

impl Actor<ActorUnprivilegedEnvironment> for DoublerActor {
    type Message = u32;

    async fn handle<'a>(
        &'a mut self,
        context: <ActorUnprivilegedEnvironment as ActorEnvironment>::HandleContext<'a, Self::Message>,
    ) -> Result<(), ActorHandleError> {
        let _ = self.root.send(2_000_000 + 2 * *context.message()).await;

        Ok(())
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////

define_unprivileged_actor!(
    DoublerActor,
    DoublerActor {
        root: ActorUnprivilegedRecipient::new(0),
    }
);
