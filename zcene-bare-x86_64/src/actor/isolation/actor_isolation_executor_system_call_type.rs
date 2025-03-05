use core::task::Poll;

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Debug)]
pub enum ActorIsolationExecutorSystemCallType {
    Continue,
    Preempt,
    Poll(Poll<()>),
    SendMessageCopy(usize, *const ()),
    Unknown(usize),
}
