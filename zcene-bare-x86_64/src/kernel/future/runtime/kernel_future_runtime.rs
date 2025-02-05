use crate::kernel::future::runtime::KernelFutureRuntimeHandler;
use zcene_core::future::runtime::FutureRuntime;

////////////////////////////////////////////////////////////////////////////////////////////////////

pub type KernelFutureRuntime = FutureRuntime<KernelFutureRuntimeHandler>;
