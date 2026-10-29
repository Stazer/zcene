pub struct UnprivilegedSystemCallEvent<SC> {
    pub identifier: usize,
    pub argument0: u64,
    pub argument1: u64,
    pub context: SC,
}

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Clone, Copy, Debug)]
pub struct UnprivilegedExceptionEvent {
    pub vector: u64,
    pub error_code: u64,
    pub instruction_pointer: u64,

    pub address: u64,
}

////////////////////////////////////////////////////////////////////////////////////////////////////

pub enum UnprivilegedEvent<SC, PC> {
    SystemCall(UnprivilegedSystemCallEvent<SC>),
    Preemption(PC),
    Exception(UnprivilegedExceptionEvent),
}
