pub enum ActorUnprivilegedSystemCallKind {
    Continue,
    Preempt,
    Complete,
    SendMessageCopy { recipient: usize, message: *const () },
    Unknown(usize),
}

unsafe impl Send for ActorUnprivilegedSystemCallKind {}

impl ActorUnprivilegedSystemCallKind {
    pub fn parse(identifier: usize, argument0: u64, argument1: u64) -> Self {
        match identifier {
            0 => Self::Continue,
            1 => Self::Preempt,
            2 => Self::Complete,
            3 => Self::SendMessageCopy {
                recipient: argument0 as usize,
                message: argument1 as *const (),
            },
            id => Self::Unknown(id),
        }
    }
}
