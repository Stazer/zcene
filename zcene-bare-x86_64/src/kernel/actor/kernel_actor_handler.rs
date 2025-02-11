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
        match specification.execution_mode {
            KernelActorExecutionMode::Privileged => self.spawn_privileged(specification),
            KernelActorExecutionMode::Unprivileged => self.spawn_unprivileged(specification),
            //_ => todo!(),
        }
    }

    fn enter(&self, specification: Self::EnterSpecification) -> Result<(), ActorEnterError> {
        self.future_runtime.run();

        Ok(())
    }
}

use core::future::Future;
use core::task::{Context, Poll};
use zcene_core::actor::ActorMessageChannelReceiver;
use core::pin::{pin, Pin};

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

pub enum Thread {
    Privileged {
    },
    Unprivileged {
    },
}

#[derive(Default)]
pub struct Scheduler {
    threads: BTreeMap<usize, Thread>,
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

struct PollContext {
}

use core::mem::replace;

impl<A> ActorExecutor<A>
where
    A: Actor<KernelActorHandler>,
{
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
                },
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
                },
                State::RunningHandle(mut state, message) => {
                    let result = {
                        let mut pinned = pin!(state.handle(
                            ActorCommonHandleContext::new(message.clone()),
                        ));
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
                },
                State::Destroy(mut state) => {
                    let mut pinned = pin!(state.destroy(()));

                    loop {
                        // TODO
                        if let Poll::Ready(_) = pinned.as_mut().poll(context) {
                            break
                        }
                    }

                    self.state = State::Destroyed;
                },
                State::Destroyed => {
                    return Poll::Ready(())
                }
                State::Unknown => {
                    return Poll::Ready(())
                }
            }
        }
    }

    fn poll_unprivileged(&mut self, context: &mut Context<'_>) -> Poll<<Self as Future>::Output> {
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
                },
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
                },
                State::RunningHandle(mut state, message) => {
                    let result = {
                        let mut pinned = pin!(state.handle(
                            ActorCommonHandleContext::new(message.clone()),
                        ));
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
                },
                State::Destroy(mut state) => {
                    let mut pinned = pin!(state.destroy(()));

                    loop {
                        // TODO
                        if let Poll::Ready(_) = pinned.as_mut().poll(context) {
                            break
                        }
                    }

                    self.state = State::Destroyed;
                },
                State::Destroyed => {
                    return Poll::Ready(())
                }
                State::Unknown => {
                    return Poll::Ready(())
                }
            }
        }
    }

        /*if let State::Created(state) =  {
            self.state = State::Running(state);
        }*/
        /*let mut pinned = pin!(self.actor.create(()));
        pinned.as_mut().poll(context);*/

        /*unsafe {
            asm!(
                "mov rsp, rbp",
                "pop rbp",
                "ret",
                options(noreturn),
            );
        }*/

                    /*unsafe {
                        asm!(
                            "call {}",
                            in(reg) Self::poll_created,
                            in("rdi") Pin::get_unchecked_mut(self),
                            in("rsi") context,
                        );
                    }

                    loop {}

        /*
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
        }*/

        /*self.scheduler.lock().threads.insert(
            0,
            Thread::Privileged {
            }
        );*/

        unsafe {
            asm!(
                "call {}",
                in(reg) Self::poll,
                in("rdi") Pin::get_unchecked_mut(self),
                in("rsi") context,
            );
        }

        Poll::Ready(())

    }*/

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

        self.future_runtime.spawn(
            ActorExecutor {
                state: State::Created(specification.actor),
                scheduler: Arc::default(),
                execution_mode: KernelActorExecutionMode::Privileged,
                scheduler2: scheduler,
                receiver,
            }
        );

        return Ok(reference);

        /*self.future_runtime.spawn(async move {
            let mut actor = specification.actor;

            let executor = KernelActorCreateExecutor {
                actor: &mut actor,
                scheduler: scheduler.clone(),
            };

            executor.await;

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

        Ok(reference)*/
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

        self.future_runtime.spawn(
            ActorExecutor {
                state: State::Created(specification.actor),
                scheduler: Arc::default(),
                execution_mode: KernelActorExecutionMode::Unprivileged,
                scheduler2: scheduler,
                receiver,
            }
        );

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

    #[naked]
    unsafe fn run() {
        naked_asm!(
            "mov rax, 0x1337",
            "syscall"
        )
    }
}

use x86::msr::{rdmsr, wrmsr, IA32_STAR, IA32_LSTAR, IA32_FMASK};
use x86_64::registers::segmentation::{CS, DS, ES, SS, SegmentSelector, Segment};
use x86_64::PrivilegeLevel;
use core::arch::asm;

#[naked]
fn syscall_entry() {
    unsafe {
        naked_asm!(
            "2:",
            "mov rbx, 0x1338",
            "jmp 2b",
            "syscall"
        )
    }
}

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
    fn run(&mut self) {
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

    if Kernel::get().actor_system().enter_default().is_err() {
        loop {}
    }

    loop {}
}

#[no_mangle]
pub unsafe extern "C" fn handle_preemption(stack_pointer: u64) -> u64 {
    Kernel::get().interrupt_manager().notify_local_end_of_interrupt();

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
