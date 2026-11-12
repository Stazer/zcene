#![no_std]
#![no_main]

////////////////////////////////////////////////////////////////////////////////////////////////////

use zcene::actor::{ActorUnprivilegedEnvironment, ActorUnprivilegedRecipient};
use zcene::core::actor::{Actor, ActorEnvironment, ActorHandleError, ActorMessageSender};
use zcene::define_unprivileged_actor;

////////////////////////////////////////////////////////////////////////////////////////////////////

pub struct CounterActor {
    count: u32,
    root: ActorUnprivilegedRecipient<u32>,
}

impl Actor<ActorUnprivilegedEnvironment> for CounterActor {
    type Message = u32;

    async fn handle<'a>(
        &'a mut self,
        _context: <ActorUnprivilegedEnvironment as ActorEnvironment>::HandleContext<'a, Self::Message>,
    ) -> Result<(), ActorHandleError> {
        self.count += 1;

        let _ = self.root.send(1_000_000 + self.count).await;

        Ok(())
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////

define_unprivileged_actor!(
    CounterActor,
    CounterActor {
        count: 0,
        root: ActorUnprivilegedRecipient::new(0),
    }
);
