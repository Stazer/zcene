#![no_std]
#![no_main]

////////////////////////////////////////////////////////////////////////////////////////////////////

use zcene::actor::{ActorIsolatedEnvironment, ActorIsolatedRecipient};
use zcene::core::actor::{Actor, ActorEnvironment, ActorHandleError, ActorMessageSender};
use zcene::define_isolated_actor;

////////////////////////////////////////////////////////////////////////////////////////////////////

pub struct CounterActor {
    count: u32,
    root: ActorIsolatedRecipient<u32>,
}

impl Actor<ActorIsolatedEnvironment> for CounterActor {
    type Message = u32;

    async fn handle<'a>(
        &'a mut self,
        _context: <ActorIsolatedEnvironment as ActorEnvironment>::HandleContext<'a, Self::Message>,
    ) -> Result<(), ActorHandleError> {
        self.count += 1;

        let _ = self.root.send(1_000_000 + self.count).await;

        Ok(())
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////

define_isolated_actor!(
    CounterActor,
    CounterActor {
        count: 0,
        root: ActorIsolatedRecipient::new(0),
    }
);
