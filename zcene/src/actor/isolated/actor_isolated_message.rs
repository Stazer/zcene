use zcene_core::actor::ActorMessage;

////////////////////////////////////////////////////////////////////////////////////////////////////

/// A message that may cross the border to an isolated actor and back. The kernel copies its raw
/// bytes, and the bytes written by an isolated actor are not to be trusted.
///
/// # Safety
///
/// Every bit pattern of `size_of::<Self>()` bytes must be a valid value of the type, and it must
/// contain no pointers or references: those would be meaningless on the other side of the border
/// (or, worse, point into the wrong address space). Primitive numbers qualify, `bool`, `char`,
/// enums and anything holding a reference or a `Box` do not.
pub unsafe trait ActorIsolatedMessage: ActorMessage + Copy {}

macro_rules! implement_actor_isolated_message {
    ($($type:ty),* $(,)?) => {
        $(unsafe impl ActorIsolatedMessage for $type {})*
    };
}

implement_actor_isolated_message!(
    (),
    u8,
    u16,
    u32,
    u64,
    u128,
    usize,
    i8,
    i16,
    i32,
    i64,
    i128,
    isize,
    f32,
    f64,
);

unsafe impl<T, const N: usize> ActorIsolatedMessage for [T; N] where T: ActorIsolatedMessage {}
