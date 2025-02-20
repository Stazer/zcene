use crate::future::runtime::FutureRuntimeSpawnError;
use core::alloc::AllocError;
use ztd::{Display, Error, From};

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Clone, Debug, Display, Error, From)]
#[From(all)]
pub enum ActorSpawnError {
    Unknown,
    MemoryAllocation(AllocError),
    FutureRuntimeSpawn(FutureRuntimeSpawnError),
}
