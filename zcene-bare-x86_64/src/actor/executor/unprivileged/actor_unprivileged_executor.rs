use crate::actor::{
    ActorUnprivilegedExecutorCreateState,
    ActorUnprivilegedExecutorReceiveState, ActorUnprivilegedExecutorState,
ActorUnprivilegedExecutorDestroyState, ActorUnprivilegedExecutorHandleState,
ActorUnprivilegedExecutorStageEvent, ActorUnprivilegedExecutorSystemCall
};
use core::future::Future;
use core::marker::PhantomData;
use core::pin::{pin, Pin};
use core::task::{Waker, Context, Poll};
use core::num::NonZero;
use pin_project::pin_project;
use zcene_core::actor::{Actor, ActorCreateError, ActorContextBuilder, ActorHandler, ActorMessageChannelReceiver};
use ztd::{Constructor, From};
use core::arch::{asm, naked_asm};

////////////////////////////////////////////////////////////////////////////////////////////////////

#[pin_project]
#[derive(Constructor)]
pub struct ActorUnprivilegedExecutor<A, B, H>
where
    A: Actor<H>,
    B: ActorContextBuilder<A, H>,
    H: ActorHandler<CreateContext = (), DestroyContext = ()>,
{
    state: Option<ActorUnprivilegedExecutorState<A, H>>,
    receiver: ActorMessageChannelReceiver<A::Message>,
    context_builder: B,
    deadline_in_milliseconds: Option<NonZero<usize>>,
    #[Constructor(default)]
    marker: PhantomData<H>,
}

impl<A, B, H> Future for ActorUnprivilegedExecutor<A, B, H>
where
    A: Actor<H>,
    B: ActorContextBuilder<A, H>,
    H: ActorHandler<CreateContext = (), DestroyContext = ()>,
{
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Self::Output> {
        loop {
            match self.state.take() {
                Some(ActorUnprivilegedExecutorState::Create(state)) => {
                    use crate::kernel::logger::println;

                    let user_stack = crate::kernel::Kernel::get()
                        .memory_manager()
                        .allocate_user_stack()
                        .unwrap()
                        .initial_memory_address()
                        .as_u64();

                    let mut actor = state.into_inner().actor;

                    let mut event = ActorUnprivilegedExecutorStageEvent::from(
                        ActorUnprivilegedExecutorSystemCall::Poll(Poll::Ready(Ok::<(), ActorCreateError>(())))
                    );

                    unsafe {
                        Self::enter(&mut actor, &mut event, user_stack, Self::create_main);
                    }

                    println!("Hey... {:?}", event);

                    loop {}
                }
                Some(ActorUnprivilegedExecutorState::Receive(state)) => {
                    let mut actor = state.into_inner().actor;

                    let result = {
                        let mut pinned = pin!(self.receiver.receive());

                        pinned.as_mut().poll(context)
                    };

                    match result {
                        Poll::Pending => {
                            self.state =
                                Some(ActorUnprivilegedExecutorReceiveState::new(actor).into());

                            return Poll::Pending;
                        }
                        Poll::Ready(None) => {
                            self.state =
                                Some(ActorUnprivilegedExecutorDestroyState::new(actor).into());
                        }
                        Poll::Ready(Some(message)) => {
                            self.state = Some(
                                ActorUnprivilegedExecutorHandleState::new(actor, message).into(),
                            );
                        }
                    }
                }
                Some(ActorUnprivilegedExecutorState::Handle(state)) => {
                    /*let ActorUnprivilegedExecutorHandleStateInner {
                        mut actor, message, ..
                    } = state.into_inner();

                    let result = {
                        let handle_context =
                            self.context_builder.build_handle_context(&actor, &message);
                        let mut pinned = pin!(actor.handle(handle_context));

                        pinned.as_mut().poll(context)
                    };

                    match result {
                        Poll::Pending => {
                            self.state =
                                Some(ActorUnprivilegedExecutorReceiveState::new(actor).into());

                            return Poll::Pending;
                        }
                        // TODO: Handle result
                        Poll::Ready(_result) => {
                            self.state =
                                Some(ActorUnprivilegedExecutorReceiveState::new(actor).into());
                        }
                    }*/
                }
                Some(ActorUnprivilegedExecutorState::Destroy(state)) => {
                    /*let mut actor = state.into_inner().actor;

                    let destroy_context = self.context_builder.build_destroy_context(&actor);
                    let mut pinned = pin!(actor.destroy(destroy_context));

                    loop {
                        // TODO: Handle result
                        if let Poll::Ready(_result) = pinned.as_mut().poll(context) {
                            break;
                        }
                    }*/
                }
                None => return Poll::Ready(()),
            }
        }
    }
}

impl<A, B, H> ActorUnprivilegedExecutor<A, B, H>
where
    A: Actor<H>,
    B: ActorContextBuilder<A, H>,
    H: ActorHandler<CreateContext = (), DestroyContext = ()>,
{
    extern "C" fn create_main(actor: &mut A) {
        let mut context = Context::from_waker(Waker::noop());
        let mut pinned = pin!(actor.create(()));

        let result = match pinned.as_mut().poll(&mut context) {
            Poll::Pending => 0,
            Poll::Ready(result) => 0,
        };

        unsafe {
            asm!(
                "mov rdi, 0x0",
                "syscall",
            )
        }
    }

    extern "C" fn handle_main(actor: &mut A) {
        let mut context = Context::from_waker(Waker::noop());
        let mut pinned = pin!(actor.create(()));

        let result = match pinned.as_mut().poll(&mut context) {
            Poll::Pending => 0,
            Poll::Ready(result) => 0,
        };

        unsafe {
            asm!(
                "mov rdi, 0x0",
                "syscall",
            )
        }
    }

    extern "C" fn destroy_main(actor: A) {
        let mut context = Context::from_waker(Waker::noop());
        let mut pinned = pin!(actor.destroy(()));

        let result = match pinned.as_mut().poll(&mut context) {
            Poll::Pending => 0,
            Poll::Ready(result) => 0,
        };

        unsafe {
            asm!(
                "mov rdi, 0x0",
                "syscall",
            )
        }
    }

    #[naked]
    unsafe extern "C" fn enter<T>(
        actor: &mut A,
        stage: &mut ActorUnprivilegedExecutorStageEvent<T>,
        stack: u64,
        function: extern "C" fn(&mut A),
    ) {
        naked_asm!(
            // Save callee-saved registers
            "push rbx",
            "push r12",
            "push r13",
            "push r14",
            "push r15",
            // Save event address
            "push rsi",
            // Save current stack address
            "mov rax, rsp",
            // Create interrupt frame for return
            "push 32 | 3",
            "push rdx",
            "push 0x000",
            "push 24 | 3",
            "push rcx",
            // Store previously saved stack address
            "mov rdx, rax",
            "shr rdx, 32",
            "mov rcx, 0xC0000102",
            "wrmsr",
            // Perform return
            "iretq",
        )
    }
}
