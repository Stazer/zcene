use crate::architecture::current_execution_unit_identifier;
use crate::kernel::actor::KernelActorThreadScheduler;
use crate::kernel::future::runtime::KernelFutureRuntimeHandler;
use crate::kernel::future::runtime::KernelFutureRuntimeReference;
use crate::kernel::logger::println;
use crate::kernel::Kernel;
use crate::kernel::TimerActorMessage;
use alloc::sync::Arc;
use core::marker::PhantomData;
use core::mem::replace;
use x86::current::registers::rsp;
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
use x86::current::rflags::RFlags;

pub use super::*;

////////////////////////////////////////////////////////////////////////////////////////////////////

use zcene_bare::memory::address::VirtualMemoryAddressPerspective;
use zcene_bare::memory::region::VirtualMemoryRegion;

pub type KernelActorInstructionRegion = VirtualMemoryRegion;

pub struct KernelActorSpawnSpecification<A, H>
where
    A: Actor<H>,
    H: ActorHandler,
{
    actor: A,
    execution_mode: KernelActorExecutionMode,
    instruction_region: KernelActorInstructionRegion,
    handler: PhantomData<H>,
}

impl<A, H> KernelActorSpawnSpecification<A, H>
where
    A: Actor<H>,
    H: ActorHandler,
{
    pub fn new(
        actor: A,
        execution_mode: KernelActorExecutionMode,
        instruction_region: KernelActorInstructionRegion,
    ) -> Self {
        Self {
            actor,
            execution_mode,
            instruction_region,
            handler: PhantomData::<H>,
        }
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////

use alloc::collections::BTreeMap;

#[derive(Constructor)]
pub struct KernelActorHandler {
    future_runtime: KernelFutureRuntimeReference,
    scheduler: Arc<Mutex<KernelActorThreadScheduler>>,
    tests: Mutex<BTreeMap<usize, usize>>,
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

    type SpawnSpecification<A>
        = KernelActorSpawnSpecification<A, Self>
    where
        A: Actor<Self>;

    type EnterSpecification = ();

    fn allocator(&self) -> &Self::Allocator {
        self.future_runtime.handler().allocator()
    }

    fn spawn<A>(
        &self,
        specification: Self::SpawnSpecification<A>,
    ) -> Result<ActorAddressReference<A, Self>, ActorSpawnError>
    where
        A: Actor<Self>,
    {
        let (sender, receiver) = ActorMessageChannel::<A::Message>::new_unbounded();

        let reference = ActorAddressReference::<A, Self>::try_new_in(
            <Self as ActorHandler>::Address::new(sender, PhantomData),
            self.allocator().clone(),
        )?;

        self.future_runtime.spawn(ActorExecutor2 {
            scheduler: Arc::default(),
            r#type: match specification.execution_mode {
                KernelActorExecutionMode::Privileged => ActorExecutorType::InternalPrivileged {
                    state: ActorExecutorState::Created(specification.actor),
                },
                KernelActorExecutionMode::Unprivileged => ActorExecutorType::InternalUnprivileged {
                    state: ActorExecutorState::Created(Box::new(specification.actor)),
                },
            },
            receiver,
        });

        Ok(reference)
    }

    fn enter(&self, specification: Self::EnterSpecification) -> Result<(), ActorEnterError> {
        self.future_runtime.run();

        Ok(())
    }
}

use core::future::Future;
use core::pin::{pin, Pin};
use core::task::{Context, Poll};
use zcene_core::actor::ActorMessageChannelReceiver;

use zcene_core::actor::ActorCreateError;

#[naked]
unsafe extern "C" fn actor_system_call_entry_point() {
    naked_asm!(
        "mov rcx, 0xC0000102",
        "rdmsr",
        "mov rsi, rax",
        "shl rdx, 32",
        "or rsi, rdx",

        "mov rsp, rsi",

        "pop rsi",

        "jmp actor_system_call_restore",
    )
}

#[no_mangle]
unsafe extern "C" fn actor_system_call_restore(
    // TODO: check size
    system_call: &ActorExecutorStageSystemCall<Result<(), ActorCreateError>>,
    event: &mut ActorExecutorStageEvent<Result<(), ActorCreateError>>,
) {
    *event = ActorExecutorStageEvent::SystemCall(system_call.clone());
}

#[naked]
pub unsafe extern "C" fn actor_deadline_entry_point() {
    naked_asm!(
        "mov rcx, 0xC0000102",
        "rdmsr",
        "mov rsi, rax",
        "shl rdx, 32",
        "or rsi, rdx",

        "mov rsp, rsi",

        "pop rdi",

        "jmp actor_deadline_restore",
    )
}

#[no_mangle]
unsafe extern "C" fn actor_deadline_restore(
    event: &mut ActorExecutorStageEvent<Result<(), ActorCreateError>>,
) {
    *event = ActorExecutorStageEvent::Preemption {
    };
}

#[naked]
unsafe extern "C" fn actor_deadline() {
    naked_asm!(
        "push rax",
        "push rcx",
        "push rdx",
        "push rbx",
        "push rbp",
        "push rsi",
        "push rdi",
        "push r8",
        "push r9",
        "push r10",
        "push r11",
        "push r12",
        "push r13",
        "push r14",
        "push r15",
        "pushfq",
    )
}

#[naked]
unsafe extern "C" fn actor_deadline2() {
    naked_asm!(
        "popfq",
        "pop r15",
        "pop r14",
        "pop r13",
        "pop r12",
        "pop r11",
        "pop r10",
        "pop r9",
        "pop r8",
        "pop rdi",
        "pop rsi",
        "pop rbp",
        "pop rbx",
        "pop rdx",
        "pop rcx",
        "pop rax",

        "ireqt",
    )
}

#[derive(Debug)]
pub enum ActorExecutorStageSystemCall<T> {
    Poll(Poll<T>),
}

impl<T> Clone for ActorExecutorStageSystemCall<T>
where
    T: Clone,
{
    fn clone(&self) -> Self {
        match self {
            Self::Poll(poll) => Self::Poll(poll.clone()),
        }
    }
}

#[derive(Debug)]
pub enum ActorExecutorStageEvent<T> {
    SystemCall(ActorExecutorStageSystemCall<T>),
    Preemption {
    },
}

use core::task::Waker;
use alloc::boxed::Box;

pub struct ActorThread {
    waker: Waker,
    return_address: u64,
}

#[derive(Default)]
pub struct Scheduler {
    threads: BTreeMap<usize, Mutex<ActorThread>>,
}

enum ActorExecutorState<A, M> {
    Created(A),
    RunningReceive(A),
    RunningHandle(A, M),
    Destroy(A),
    Destroyed,
    Unknown,
}

pub enum ActorExecutorType<A>
where
    A: Actor<KernelActorHandler>,
{
    InternalPrivileged {
        state: ActorExecutorState<A, A::Message>,
    },
    InternalUnprivileged {
        state: ActorExecutorState<Box<A>, A::Message>,
    },
}

#[pin_project::pin_project]
pub struct ActorExecutor2<A>
where
    A: Actor<KernelActorHandler>,
{
    r#type: ActorExecutorType<A>,
    scheduler: Arc<Mutex<Scheduler>>,
    receiver: ActorMessageChannelReceiver<A::Message>,
}

impl<A> ActorExecutor2<A>
where
    A: Actor<KernelActorHandler>,
{
    fn poll_internal_privileged(
        state: &mut ActorExecutorState<A, A::Message>,
        context: &mut Context<'_>,
        receiver: &mut ActorMessageChannelReceiver<A::Message>,
    ) -> Poll<<Self as Future>::Output> {
        loop {
            match replace(state, ActorExecutorState::Unknown) {
                ActorExecutorState::Created(mut actor) => {
                    let result = {
                        let mut pinned = pin!(actor.create(()));
                        pinned.as_mut().poll(context)
                    };

                    match result {
                        Poll::Pending => {
                            *state = ActorExecutorState::Created(actor);
                            return Poll::Pending;
                        }
                        // TODO: handle result
                        Poll::Ready(_result) => {
                            *state = ActorExecutorState::RunningReceive(actor);
                        }
                    }
                }
                ActorExecutorState::RunningReceive(mut actor) => {
                    let result = {
                        let mut pinned = pin!(receiver.receive());
                        pinned.as_mut().poll(context)
                    };

                    match result {
                        Poll::Pending => {
                            *state = ActorExecutorState::RunningReceive(actor);
                            return Poll::Pending;
                        }
                        Poll::Ready(Some(message)) => {
                            *state = ActorExecutorState::RunningHandle(actor, message);
                        }
                        Poll::Ready(None) => {
                            *state = ActorExecutorState::Destroy(actor);
                        }
                    }
                }
                ActorExecutorState::RunningHandle(mut actor, message) => {
                    let result = {
                        let mut pinned =
                            pin!(actor.handle(ActorCommonHandleContext::new(message.clone()),));
                        pinned.as_mut().poll(context)
                    };

                    match result {
                        Poll::Pending => {
                            *state = ActorExecutorState::RunningHandle(actor, message);
                            return Poll::Pending;
                        }
                        Poll::Ready(message) => {
                            *state = ActorExecutorState::RunningReceive(actor);
                        }
                    }
                }
                ActorExecutorState::Destroy(mut actor) => {
                    let mut pinned = pin!(actor.destroy(()));

                    loop {
                        // TODO
                        if let Poll::Ready(_) = pinned.as_mut().poll(context) {
                            break;
                        }
                    }

                    *state = ActorExecutorState::Destroyed;
                }
                ActorExecutorState::Destroyed => return Poll::Ready(()),
                ActorExecutorState::Unknown => return Poll::Ready(()),
            }
        }

        Poll::Ready(())
    }

    extern "C" fn execute(actor: &mut A) -> ! {
        let mut context = Context::from_waker(Waker::noop());

        let mut pinned = pin!(actor.create(()));

        let result = match pinned.as_mut().poll(&mut context) {
            Poll::Pending => {
                ActorExecutorStageSystemCall::Poll(Poll::Pending)
            },
            Poll::Ready(result) => {
                ActorExecutorStageSystemCall::Poll(Poll::Ready(result))
            }
        };

        unsafe {
            asm!(
                "mov rdi, {}",
                "syscall",

                in(reg) &result,

                options(noreturn),
            )
        }
    }

    #[naked]
    unsafe extern "C" fn system_return<T>(
        user_stack: u64, // rdi
        function: u64, // rsi
        actor: &mut A, // rdx
        result: &mut ActorExecutorStageEvent<T>, // rcx
        size: u64, // r8
        //rflags: RFlags, // r9
    ) {
        naked_asm!(
            "push rcx",

            "mov rcx, 0xC0000102",
            "mov rax, rsp",
            "mov rdx, rax",
            "shr rdx, 32",
            "wrmsr",

            "mov rsp, rdi",
            "mov rcx, rsi",
            "or r11, 0x200",

            "sysretq",
        )
    }

    fn poll_internal_unprivileged(
        state: &mut ActorExecutorState<Box<A>, A::Message>,
        context: &mut Context<'_>,
        receiver: &mut ActorMessageChannelReceiver<A::Message>,
    ) -> Poll<<Self as Future>::Output> {
        loop {
            match replace(state, ActorExecutorState::Unknown) {
                ActorExecutorState::Created(mut actor) => {
                    use x86_64::structures::gdt::GlobalDescriptorTable;
                    use x86_64::structures::gdt::Descriptor;
                    use x86_64::VirtAddr;
                    use x86_64::structures::tss::TaskStateSegment;
                    use x86_64::instructions::tables::load_tss;

                    let mut gdt = GlobalDescriptorTable::new();

                    let mut tss = TaskStateSegment::new();
                    tss.privilege_stack_table[0] = VirtAddr::new(rsp() - 0x1000);

                    let kernel_code = gdt.append(Descriptor::kernel_code_segment());
                    let kernel_data = gdt.append(Descriptor::kernel_data_segment());
                    let user_code = gdt.append(Descriptor::user_code_segment());
                    gdt.append(Descriptor::user_data_segment());

                    let tss_descriptor = unsafe {
                        gdt.append(Descriptor::tss_segment_unchecked(&tss))
                    };
                    //RDI=000000000362bfd8;
                    // RCX=0000010000004837 <---
                    // RSP=0000010000004820

                    unsafe {
                        gdt.load_unsafe();

                        CS::set_reg(kernel_code);
                        DS::set_reg(kernel_data);

                        load_tss(tss_descriptor);

                        wrmsr(IA32_STAR, (u64::from(kernel_code.0) << 32) | (u64::from(user_code.0) << 48));
                        wrmsr(IA32_LSTAR, actor_system_call_entry_point as u64);
                        wrmsr(IA32_FMASK, 0);
                    }

                    let user_stack = Kernel::get()
                        .memory_manager()
                        .allocate_user_stack()
                        .unwrap()
                        .initial_memory_address()
                        .as_u64();

                    Kernel::get().interrupt_manager().reset_oneshot(core::time::Duration::from_millis(100));

                    crate::kernel::logger::println!("before {:X}", user_stack);

                    let mut result = ActorExecutorStageEvent::Preemption {};

                    unsafe {
                        Self::system_return::<Result<(), ActorCreateError>>(
                            user_stack,
                            Self::execute as _,
                            &mut *actor,
                            &mut result,
                            size_of::<ActorExecutorStageEvent<Result<(), ActorCreateError>>>() as u64,
                            //RFlags::empty() | RFlags::FLAGS_IF,
                        );
                    };

                    crate::kernel::logger::println!("after call {:?}", result);

                    loop {}
                }
                ActorExecutorState::RunningReceive(mut actor) => {
                    let result = {
                        let mut pinned = pin!(receiver.receive());
                        pinned.as_mut().poll(context)
                    };

                    match result {
                        Poll::Pending => {
                            *state = ActorExecutorState::RunningReceive(actor);
                            return Poll::Pending;
                        }
                        Poll::Ready(Some(message)) => {
                            *state = ActorExecutorState::RunningHandle(actor, message);
                        }
                        Poll::Ready(None) => {
                            *state = ActorExecutorState::Destroy(actor);
                        }
                    }
                }
                ActorExecutorState::RunningHandle(mut actor, message) => {
                    let result = {
                        let mut pinned =
                            pin!(actor.handle(ActorCommonHandleContext::new(message.clone())));

                        pinned.as_mut().poll(context)
                    };

                    match result {
                        Poll::Pending => {
                            *state = ActorExecutorState::RunningHandle(actor, message);
                            return Poll::Pending;
                        }
                        Poll::Ready(message) => {
                            *state = ActorExecutorState::RunningReceive(actor);
                        }
                    }
                }
                ActorExecutorState::Destroy(mut actor) => {
                    let mut pinned = pin!(actor.destroy(()));

                    loop {
                        // TODO
                        if let Poll::Ready(_) = pinned.as_mut().poll(context) {
                            break;
                        }
                    }

                    *state = ActorExecutorState::Destroyed;
                }
                ActorExecutorState::Destroyed => return Poll::Ready(()),
                ActorExecutorState::Unknown => return Poll::Ready(()),
            }
        }

        Poll::Ready(())
    }
}

impl<A> Future for ActorExecutor2<A>
where
    A: Actor<KernelActorHandler>,
{
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Self::Output> {
        let Self {
            receiver, r#type, ..
        } = &mut *self;

        match r#type {
            ActorExecutorType::InternalPrivileged { state } => {
                Self::poll_internal_privileged(state, context, receiver)
            }
            ActorExecutorType::InternalUnprivileged { state } => {
                Self::poll_internal_unprivileged(state, context, receiver)
            }
        }
    }
}

impl KernelActorHandler {
    /*pub fn reschedule(&self, stack_pointer: VirtualMemoryAddress) -> VirtualMemoryAddress {
        let mut scheduler = self.scheduler.lock();

        let next_thread = scheduler.queue_mut().pop_front();

        scheduler.r#break(
            current_execution_unit_identifier(),
            VirtualMemoryAddress::from(stack_pointer),
        );

        scheduler.r#continue(current_execution_unit_identifier(), next_thread)
    }*/
}

use core::arch::asm;
use x86::msr::{rdmsr, wrmsr, IA32_FMASK, IA32_LSTAR, IA32_STAR};
use x86_64::registers::segmentation::{Segment, SegmentSelector, CS, DS, ES, SS};
use x86_64::PrivilegeLevel;

impl ActorDiscoveryHandler for KernelActorHandler {
    fn discover<M>(&self) -> Option<ActorMailbox<M, Self>>
    where
        M: ActorMessage,
    {
        None
    }
}

/*pub fn hello() -> ! {
    x86_64::instructions::interrupts::enable();

    if Kernel::get().actor_system().enter_default().is_err() {
        loop {}
    }

    loop {}
}

#[no_mangle]
pub unsafe extern "C" fn handle_preemption(stack_pointer: u64) -> u64 {
    Kernel::get()
        .interrupt_manager()
        .notify_local_end_of_interrupt();

    stack_pointer
}*/

use core::arch::naked_asm;

/*#[naked]
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
}*/
