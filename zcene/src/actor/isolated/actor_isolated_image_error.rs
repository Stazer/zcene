#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActorIsolatedImageError {
    NotElf,
    Misaligned,
    Unsupported,
    Malformed,
    TooManySegments,
    WritableAndExecutable,
}
