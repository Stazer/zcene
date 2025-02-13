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

        /*match specification.execution_mode {
            KernelActorExecutionMode::Privileged => self.spawn_privileged(specification),
            KernelActorExecutionMode::Unprivileged => self.spawn_unprivileged(specification),
            //_ => todo!(),
        }*/
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

#[pin_project::pin_project]
pub struct KernelActorCreateExecutor<'a, A>
where
    A: Actor<KernelActorHandler>,
{
    actor: &'a mut A,
    scheduler: Arc<Mutex<KernelActorThreadScheduler>>,
}

impl<'a, A> Future for KernelActorCreateExecutor<'a, A>
where
    A: Actor<KernelActorHandler>,
{
    type Output = Result<(), ActorCreateError>;

    fn poll(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Self::Output> {
        let scheduler = self.scheduler.clone();

        without_interrupts(|| {
            scheduler.lock().begin(current_execution_unit_identifier());
        });

        let mut pinned = pin!(self.actor.create(()));
        let result = pinned.as_mut().poll(context);

        without_interrupts(|| {
            scheduler.lock().end(current_execution_unit_identifier());
        });

        result
    }
}

#[repr(u64)]
#[derive(Copy, Clone, Debug)]
pub enum ActorExecutorStageResult {
    Ready,
    Pending,
    Preempted,
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

    unsafe fn preempt() {
        loop {}
    }

    #[naked]
    unsafe extern "C" fn system_call(x0: u64) {
        naked_asm!(
            "mov rcx, 0xC0000102",
            "rdmsr",
            "mov rsi, rax",
            "shl rdx, 32",
            "or rsi, rdx",

            "mov rsp, rsi",

            "mov rax, 0",

            "ret",
        )
    }

    unsafe extern "C" fn system_return(actor: &mut A, context: &mut Context<'_>) {
        let mut pinned = pin!(actor.create(()));
        pinned.as_mut().poll(context);

        loop {}
    }

    #[naked]
    unsafe extern "C" fn execute(
        user_stack: u64,
        function: u64,
        actor: &mut A,
        context: &mut Context<'_>,
    ) -> ActorExecutorStageResult {
        naked_asm!(
            "mov rcx, 0xC0000102",
            "mov rax, rsp",
            "mov rdx, rax",
            "shr rdx, 32",
            "wrmsr",

            "mov rsp, rdi",
            "mov rcx, rsi",
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
                    unsafe {
                        wrmsr(IA32_STAR, (0x08u64 << 32) | (0x1Bu64 << 48));
                        wrmsr(IA32_LSTAR, Self::system_call as u64);
                        wrmsr(IA32_FMASK, 0);
                    }

                    let user_stack = Kernel::get().memory_manager().allocate_user_stack().unwrap().initial_memory_address().as_u64();

                    crate::kernel::logger::println!("before {:X}", user_stack);

                    let result = unsafe {
                        Self::execute(
                            user_stack,
                            Self::system_return as _,
                            &mut *actor,
                            context,
                        )
                    };

                    crate::kernel::logger::println!("after call {:?}", result);

                    /*let result = {
                        let mut pinned = pin!(actor.create(()));
                        pinned.as_mut().poll(context)
                    };*/

                    /*match result {
                        Poll::Pending => {
                            *state = ActorExecutorState::Created(actor);
                            return Poll::Pending;
                        }
                        // TODO: handle result
                        Poll::Ready(_result) => {
                            *state = ActorExecutorState::RunningReceive(actor);
                        }
                    }*/

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

enum State2<A>
where
    A: Actor<KernelActorHandler>,
{
    Created(Box<A>),
    RunningReceive(Box<A>),
    RunningHandle(Box<A>, A::Message),
    Destroy(Box<A>),
    Destroyed,
    Unknown,
}

enum State<A>
where
    A: Actor<KernelActorHandler>,
{
    Created(A),
    RunningReceive(A),
    RunningHandle(A, A::Message),
    Destroy(A),
    Destroyed,
    Unknown,
}

#[pin_project::pin_project]
pub struct ActorExecutor<A>
where
    A: Actor<KernelActorHandler>,
{
    scheduler: Arc<Mutex<Scheduler>>,
    execution_mode: KernelActorExecutionMode,
    scheduler2: Arc<Mutex<KernelActorThreadScheduler>>,
    receiver: ActorMessageChannelReceiver<A::Message>,
    state: State<A>,
}

impl<A> ActorExecutor<A>
where
    A: Actor<KernelActorHandler>,
{
    #[naked]
    unsafe fn run() -> ! {
        naked_asm!("syscall")
    }

    unsafe fn execute(sp: u64) -> ! {
        asm!(
            //"mov rsp, rdi",
            "mov rcx, {instruction_pointer}",

            "sysretq",

            instruction_pointer = in(reg) Self::run as u64,
            options(noreturn),
        )
    }

    unsafe fn syscall_entry() {
        asm!("pop rax",);

        crate::kernel::logger::println!("syscall");
    }

    fn poll_unprivileged(&mut self, context: &mut Context<'_>) -> Poll<<Self as Future>::Output> {
        loop {
            match replace(&mut self.state, State::Unknown) {
                State::Created(mut state) => {
                    crate::kernel::logger::println!("before");

                    let user_stack = Kernel::get()
                        .memory_manager()
                        .allocate_user_stack()
                        .unwrap();

                    unsafe {
                        wrmsr(IA32_STAR, (0x08u64 << 32) | (0x1Bu64 << 48));
                        wrmsr(IA32_LSTAR, Self::syscall_entry as u64);
                        wrmsr(IA32_FMASK, 0);
                    }

                    unsafe {
                        asm!(
                            "call {}",
                            in(reg) Self::execute,
                            in("rdi") user_stack.initial_memory_address().as_u64(),
                            //in("rsi") context,
                        );
                    }

                    crate::kernel::logger::println!("after");

                    let result = {
                        let mut pinned = pin!(state.create(()));
                        pinned.as_mut().poll(context)
                    };

                    match result {
                        Poll::Pending => {
                            self.state = State::Created(state);
                            return Poll::Pending;
                        }
                        // TODO
                        Poll::Ready(_result) => {
                            self.state = State::RunningReceive(state);
                        }
                    }
                }
                State::RunningReceive(mut state) => {
                    let result = {
                        let mut pinned = pin!(self.receiver.receive());
                        pinned.as_mut().poll(context)
                    };

                    match result {
                        Poll::Pending => {
                            self.state = State::RunningReceive(state);
                            return Poll::Pending;
                        }
                        Poll::Ready(Some(message)) => {
                            self.state = State::RunningHandle(state, message);
                        }
                        Poll::Ready(None) => {
                            self.state = State::Destroy(state);
                        }
                    }
                }
                State::RunningHandle(mut state, message) => {
                    let result = {
                        let mut pinned =
                            pin!(state.handle(ActorCommonHandleContext::new(message.clone()),));
                        pinned.as_mut().poll(context)
                    };

                    match result {
                        Poll::Pending => {
                            self.state = State::RunningHandle(state, message);
                            return Poll::Pending;
                        }
                        Poll::Ready(message) => {
                            self.state = State::RunningReceive(state);
                        }
                    }
                }
                State::Destroy(mut state) => {
                    let mut pinned = pin!(state.destroy(()));

                    loop {
                        // TODO
                        if let Poll::Ready(_) = pinned.as_mut().poll(context) {
                            break;
                        }
                    }

                    self.state = State::Destroyed;
                }
                State::Destroyed => return Poll::Ready(()),
                State::Unknown => return Poll::Ready(()),
            }
        }
    }

    fn poll_privileged(&mut self, context: &mut Context<'_>) -> Poll<<Self as Future>::Output> {
        loop {
            match replace(&mut self.state, State::Unknown) {
                State::Created(mut state) => {
                    let result = {
                        let mut pinned = pin!(state.create(()));
                        pinned.as_mut().poll(context)
                    };

                    match result {
                        Poll::Pending => {
                            self.state = State::Created(state);
                            return Poll::Pending;
                        }
                        // TODO
                        Poll::Ready(_result) => {
                            self.state = State::RunningReceive(state);
                        }
                    }
                }
                State::RunningReceive(mut state) => {
                    let result = {
                        let mut pinned = pin!(self.receiver.receive());
                        pinned.as_mut().poll(context)
                    };

                    match result {
                        Poll::Pending => {
                            self.state = State::RunningReceive(state);
                            return Poll::Pending;
                        }
                        Poll::Ready(Some(message)) => {
                            self.state = State::RunningHandle(state, message);
                        }
                        Poll::Ready(None) => {
                            self.state = State::Destroy(state);
                        }
                    }
                }
                State::RunningHandle(mut state, message) => {
                    let result = {
                        let mut pinned =
                            pin!(state.handle(ActorCommonHandleContext::new(message.clone()),));
                        pinned.as_mut().poll(context)
                    };

                    match result {
                        Poll::Pending => {
                            self.state = State::RunningHandle(state, message);
                            return Poll::Pending;
                        }
                        Poll::Ready(message) => {
                            self.state = State::RunningReceive(state);
                        }
                    }
                }
                State::Destroy(mut state) => {
                    let mut pinned = pin!(state.destroy(()));

                    loop {
                        // TODO
                        if let Poll::Ready(_) = pinned.as_mut().poll(context) {
                            break;
                        }
                    }

                    self.state = State::Destroyed;
                }
                State::Destroyed => return Poll::Ready(()),
                State::Unknown => return Poll::Ready(()),
            }
        }
    }
}

impl<A> Future for ActorExecutor<A>
where
    A: Actor<KernelActorHandler>,
{
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Self::Output> {
        match self.execution_mode {
            KernelActorExecutionMode::Privileged => self.poll_privileged(context),
            KernelActorExecutionMode::Unprivileged => self.poll_unprivileged(context),
        }
    }
}

impl KernelActorHandler {
    fn spawn_privileged<A>(
        &self,
        specification: <Self as ActorHandler>::SpawnSpecification<A>,
    ) -> Result<ActorAddressReference<A, Self>, ActorSpawnError>
    where
        A: Actor<Self>,
    {
        let (sender, receiver) = ActorMessageChannel::<A::Message>::new_unbounded();

        let reference = ActorAddressReference::<A, Self>::try_new_in(
            <Self as ActorHandler>::Address::new(sender, PhantomData),
            self.allocator().clone(),
        )?;

        let scheduler = self.scheduler.clone();

        /*self.future_runtime.spawn(
            ActorExecutor2 {
                state: State2::Created(Box::new(specification.actor)),
                scheduler: Arc::default(),
                execution_mode: KernelActorExecutionMode::Privileged,
                scheduler2: scheduler,
                receiver,
            }
        );*/

        return Ok(reference);
    }

    fn spawn_unprivileged<A>(
        &self,
        specification: <Self as ActorHandler>::SpawnSpecification<A>,
    ) -> Result<ActorAddressReference<A, Self>, ActorSpawnError>
    where
        A: Actor<Self>,
    {
        let (sender, receiver) = ActorMessageChannel::<A::Message>::new_unbounded();

        let reference = ActorAddressReference::<A, Self>::try_new_in(
            <Self as ActorHandler>::Address::new(sender, PhantomData),
            self.allocator().clone(),
        )?;

        let scheduler = self.scheduler.clone();

        /*self.future_runtime.spawn(
            ActorExecutor2 {
                state: State2::Created(Box::new(specification.actor)),
                scheduler: Arc::default(),
                execution_mode: KernelActorExecutionMode::Unprivileged,
                scheduler2: scheduler,
                receiver,
            }
        );*/

        Ok(reference)

        /*unsafe {
            Arc::increment_strong_count(&reference);
        }

        return Ok(reference);


        let (sender, receiver) = ActorMessageChannel::<A::Message>::new_unbounded();

        let reference = ActorAddressReference::<A, Self>::try_new_in(
            <Self as ActorHandler>::Address::new(sender, PhantomData),
            self.allocator().clone(),
        )?;

        let scheduler = self.scheduler.clone();

        use alloc::boxed::Box;

        self.future_runtime.spawn(async move {
            let wrapper = Box::<dyn UnprivilegedWrapper>::from(Box::new(Wrapper {
                actor: specification.actor,
                handler: PhantomData::<Self>,
            }));

            //let create = (wrapper.create) as *const ();

            let wrapper = Box::into_raw(wrapper);

            let mut a = unsafe { Box::<dyn UnprivilegedWrapper>::from_raw(wrapper) };

            let user_stack = Kernel::get().memory_manager().allocate_user_stack().unwrap();

            unsafe {
                wrmsr(IA32_STAR, (0x08u64 << 32) | (0x1Bu64 << 48));
                wrmsr(IA32_LSTAR, syscall_entry as u64);
                wrmsr(IA32_FMASK, 0);

                // crate::kernel::logger::println!("{:X}", unsafe { rdmsr(IA32_STAR) });

                core::arch::asm!(
                    "mov rsp, {stack_pointer}",
                    "mov rcx, {instruction_pointer}",
                    "sysretq",

                    stack_pointer = in(reg) user_stack.initial_memory_address().as_u64(),
                    instruction_pointer = in(reg) Self::run as u64,
                )
            }

            //a.run();

            //let create = (*wrapper).create as u64;
        });

        Ok(reference)*/
    }

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

use core::arch::asm;
use x86::msr::{rdmsr, wrmsr, IA32_FMASK, IA32_LSTAR, IA32_STAR};
use x86_64::registers::segmentation::{Segment, SegmentSelector, CS, DS, ES, SS};
use x86_64::PrivilegeLevel;

pub trait UnprivilegedWrapper {
    fn run(&mut self);
}

pub struct Wrapper<A, H>
where
    A: Actor<H>,
    H: ActorHandler,
{
    actor: A,
    handler: PhantomData<H>,
}

impl<A, H> UnprivilegedWrapper for Wrapper<A, H>
where
    A: Actor<H>,
    H: ActorHandler,
{
    fn run(&mut self) {}
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

    /*if let Err(error) = Kernel::get()
        .timer_actor()
        .send(TimerActorMessage::Tick)
        .complete()
    {
        println!("{}", error);
    }*/

    /*let stack_pointer = Kernel::get()
        .actor_system()
        .handler()
        .reschedule(stack_pointer.into());*/

    stack_pointer
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
