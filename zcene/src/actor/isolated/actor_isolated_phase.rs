/// Passed to an image's entry point in `rdi`, with the address of the message (or 0) in `rsi`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u64)]
pub enum ActorIsolatedPhase {
    Create = 0,
    Handle = 1,
    Destroy = 2,
}
