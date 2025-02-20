use core::fmt::{self, Formatter, Debug};
use crate::actor::ActorUnprivilegedExecutorSystemCall;
use ztd::From;

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(From)]
#[From(unnamed)]
pub enum ActorUnprivilegedExecutorStageEvent<T> {
    SystemCall(ActorUnprivilegedExecutorSystemCall<T>),
    Preemption,
    Exception,
}

impl<T> Debug for ActorUnprivilegedExecutorStageEvent<T>
where
    T: Debug
{
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::SystemCall(system_call) => formatter.debug_tuple("SystemCall").field(system_call).finish(),
            Self::Preemption => write!(formatter, "Preemption"),
            Self::Exception => write!(formatter, "Exception"),
        }
    }
}
