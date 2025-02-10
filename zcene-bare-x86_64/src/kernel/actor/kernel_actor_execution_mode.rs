#[derive(Debug)]
pub enum KernelActorExecutionMode {
    Privileged,
    Unprivileged,
    InternPrivileged,
    InternUnprivileged,
    ExternPrivileged,
    ExternUnprivileged,
}
