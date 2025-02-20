use core::fmt::{self, Formatter, Debug};
use core::task::Poll;

////////////////////////////////////////////////////////////////////////////////////////////////////

pub enum ActorUnprivilegedExecutorSystemCall<T> {
    Poll(Poll<T>)
}

impl<T> Debug for ActorUnprivilegedExecutorSystemCall<T>
where
    T: Debug
{
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Poll(poll) => formatter.debug_tuple("Poll").field(poll).finish(),
        }
    }
}
