use core::task::Poll;

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Debug)]
pub enum ActorIsolationExecutorSystemCallType {
    Continue,
    Preempt,
    Poll(Poll<()>),
    SendMessageCopy(()),
    Unknown(usize),
}
