use core::cell::SyncUnsafeCell;
use x86_64::structures::idt::InterruptDescriptorTable;

////////////////////////////////////////////////////////////////////////////////////////////////////

pub static INTERRUPT_DESCRIPTOR_TABLE: SyncUnsafeCell<InterruptDescriptorTable> =
    SyncUnsafeCell::new(InterruptDescriptorTable::new());
