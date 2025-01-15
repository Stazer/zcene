use core::cell::SyncUnsafeCell;
use x86_64::structures::idt::InterruptDescriptorTable;

////////////////////////////////////////////////////////////////////////////////////////////////////

pub static INTERRUPT_DESCRIPTOR_TABLE: SyncUnsafeCell<InterruptDescriptorTable> =
    SyncUnsafeCell::new(InterruptDescriptorTable::new());

pub enum InterruptActorMessage {
    Load,
}

pub struct InterruptActor {
    interrupt_descriptor_table: InterruptDescriptorTable,
}

use zcene_core::actor::{Actor, ActorHandler};

impl Actor<H> for InterruptActor
where
    H: ActorHandler,
{
    type Message = InterruptActorMessage;


}
