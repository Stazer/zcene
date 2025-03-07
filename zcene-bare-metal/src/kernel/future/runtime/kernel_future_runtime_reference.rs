use crate::kernel::future::runtime::KernelFutureRuntimeHandler;
use zcene_core::future::runtime::FutureRuntimeReference;

////////////////////////////////////////////////////////////////////////////////////////////////////

pub type KernelFutureRuntimeReference = FutureRuntimeReference<KernelFutureRuntimeHandler>;
