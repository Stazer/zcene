use zcene_core::actor::ActorMessage;

////////////////////////////////////////////////////////////////////////////////////////////////////

/// A message that may cross the border to an unprivileged actor and back. The kernel copies its raw
/// bytes, and the bytes written by an unprivileged actor are not to be trusted.
///
/// # Safety
///
/// Every bit pattern of `size_of::<Self>()` bytes must be a valid value of the type, and it must
/// contain no pointers or references: those would be meaningless on the other side of the border
/// (or, worse, point into the wrong address space). Primitive numbers qualify, `bool`, `char`,
/// enums and anything holding a reference or a `Box` do not.
pub unsafe trait ActorUnprivilegedMessage: ActorMessage + Copy {}

macro_rules! implement_actor_unprivileged_message {
    ($($type:ty),* $(,)?) => {
        $(unsafe impl ActorUnprivilegedMessage for $type {})*
    };
}

implement_actor_unprivileged_message!(
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

unsafe impl<T, const N: usize> ActorUnprivilegedMessage for [T; N] where T: ActorUnprivilegedMessage {}
