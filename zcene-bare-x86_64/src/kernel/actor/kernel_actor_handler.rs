use crate::architecture::current_execution_unit_identifier;
use crate::kernel::actor::KernelActorThreadScheduler;
use crate::kernel::future::runtime::KernelFutureRuntimeHandler;
use crate::kernel::future::runtime::KernelFutureRuntimeReference;
use crate::kernel::logger::println;
use crate::kernel::Kernel;
use crate::kernel::TimerActorMessage;
use alloc::sync::Arc;
use core::marker::PhantomData;
use x86_64::instructions::interrupts::without_interrupts;
use zcene_bare::memory::address::PhysicalMemoryAddress;
use zcene_bare::memory::address::VirtualMemoryAddress;
use zcene_bare::synchronization::Mutex;
use zcene_core::actor::ActorMessageSender;
use zcene_core::actor::{
    Actor, ActorAddressReference, ActorCommonHandleContext, ActorDiscoveryHandler, ActorEnterError,
    ActorHandler, ActorMailbox, ActorMessage, ActorMessageChannel, ActorMessageChannelAddress,
    ActorSpawnError,
};
use zcene_core::future::runtime::FutureRuntimeHandler;
use zcene_core::future::FutureExt;
use ztd::Constructor;

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Constructor)]
pub struct KernelActorHandler {
    future_runtime: KernelFutureRuntimeReference,
    scheduler: Arc<Mutex<KernelActorThreadScheduler>>,
}

impl ActorHandler for KernelActorHandler {
    type Address<A>
        = ActorMessageChannelAddress<A, Self>
    where
        A: Actor<Self>;

    type Allocator = <KernelFutureRuntimeHandler as FutureRuntimeHandler>::Allocator;

    type CreateContext = ();
    type HandleContext<M>
        = ActorCommonHandleContext<M>
    where
        M: ActorMessage;
    type DestroyContext = ();

    fn allocator(&self) -> &Self::Allocator {
        self.future_runtime.handler().allocator()
    }

    fn spawn<A>(&self, actor: A) -> Result<ActorAddressReference<A, Self>, ActorSpawnError>
    where
        A: Actor<Self>,
    {
        let (sender, receiver) = ActorMessageChannel::<A::Message>::new_unbounded();

        let reference = ActorAddressReference::<A, Self>::try_new_in(
            Self::Address::new(sender, PhantomData),
            self.allocator().clone(),
        )?;

        let scheduler = self.scheduler.clone();

        self.future_runtime.spawn(async move {
            let mut actor = actor;

            without_interrupts(|| {
                scheduler.lock().begin(current_execution_unit_identifier());
            });

            actor.create(()).await;

            without_interrupts(|| {
                scheduler.lock().end(current_execution_unit_identifier());
            });

            loop {
                let message = match receiver.receive().await {
                    Some(message) => message,
                    None => break,
                };

                without_interrupts(|| {
                    scheduler.lock().begin(current_execution_unit_identifier());
                });

                actor.handle(ActorCommonHandleContext::new(message)).await;

                without_interrupts(|| {
                    scheduler.lock().end(current_execution_unit_identifier());
                });
            }

            without_interrupts(|| {
                scheduler.lock().begin(current_execution_unit_identifier());
            });

            actor.destroy(()).await;

            without_interrupts(|| {
                scheduler.lock().end(current_execution_unit_identifier());
            });
        });

        Ok(reference)
    }

    fn enter(&self) -> Result<(), ActorEnterError> {
        self.future_runtime.run();

        Ok(())
    }
}

impl KernelActorHandler {
    pub fn reschedule(&self, stack_pointer: VirtualMemoryAddress) -> VirtualMemoryAddress {
        let mut scheduler = self.scheduler.lock();

        let next_thread = scheduler.queue_mut().pop_front();

        scheduler.r#break(
            current_execution_unit_identifier(),
            VirtualMemoryAddress::from(stack_pointer),
        );

        scheduler.r#continue(current_execution_unit_identifier(), next_thread)
    }
}

impl ActorDiscoveryHandler for KernelActorHandler {
    fn discover<M>(&self) -> Option<ActorMailbox<M, Self>>
    where
        M: ActorMessage,
    {
        None
    }
}

pub fn hello() -> ! {
    x86_64::instructions::interrupts::enable();

    if Kernel::get().actor_system().enter().is_err() {
        loop {}
    }

    loop {}
}

#[no_mangle]
pub unsafe extern "C" fn handle_preemption(stack_pointer: u64) -> u64 {
    let apic_ptr = Kernel::get()
        .memory_manager()
        .translate_physical_memory_address(PhysicalMemoryAddress::from(
            x86::msr::rdmsr(x86::msr::APIC_BASE) & 0xFFFFF000,
        ))
        .cast_mut::<u32>();

    unsafe {
        use core::ptr;
        ptr::write_volatile(apic_ptr.add(0x0B0 / 4), 0);
    }

    if let Err(error) = Kernel::get()
        .timer_actor()
        .send(TimerActorMessage::Tick)
        .complete()
    {
        println!("{}", error);
    }

    let stack_pointer = Kernel::get()
        .actor_system()
        .handler()
        .reschedule(stack_pointer.into());

    stack_pointer.as_u64()
}

use core::arch::naked_asm;

#[naked]
#[no_mangle]
pub unsafe fn timer_entry_point() {
    naked_asm!(
        "push rax",
        "push rbx",
        "push rcx",
        "push rdx",
        "push rsi",
        "push rdi",
        "push rbp",
        "push r8",
        "push r9",
        "push r10",
        "push r11",
        "push r12",
        "push r13",
        "push r14",
        "push r15",
        "mov rdi, rsp",
        "cld",
        "call handle_preemption",
        "mov rsp, rax",
        "pop r15",
        "pop r14",
        "pop r13",
        "pop r12",
        "pop r11",
        "pop r10",
        "pop r9",
        "pop r8",
        "pop rbp",
        "pop rdi",
        "pop rsi",
        "pop rdx",
        "pop rcx",
        "pop rbx",
        "pop rax",
        "iretq",
    );
}
