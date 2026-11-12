#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActorUnprivilegedImageError {
    NotElf,
    Misaligned,
    Unsupported,
    Malformed,
    TooManySegments,
    WritableAndExecutable,
}
