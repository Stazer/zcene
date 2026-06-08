pub struct UnprivilegedSystemCallEvent<SC> {
    pub identifier: usize,
    pub argument0: u64,
    pub argument1: u64,
    pub context: SC,
}

////////////////////////////////////////////////////////////////////////////////////////////////////

pub enum UnprivilegedEvent<SC, PC> {
    SystemCall(UnprivilegedSystemCallEvent<SC>),
    Preemption(PC),
    Exception,
}
