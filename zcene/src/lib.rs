#![feature(abi_x86_interrupt)]
#![feature(adt_const_params)]
#![feature(allocator_ext)]
#![feature(arbitrary_self_types)]
#![feature(btreemap_alloc)]
#![feature(step_trait)]
#![feature(stmt_expr_attributes)]
#![feature(sync_unsafe_cell)]
#![cfg_attr(not(feature = "hosted"), no_std)]

////////////////////////////////////////////////////////////////////////////////////////////////////

//#[cfg(test)]
//#[macro_use]
//extern crate std;

////////////////////////////////////////////////////////////////////////////////////////////////////

extern crate alloc;

////////////////////////////////////////////////////////////////////////////////////////////////////

pub mod actor;
pub mod platform;
pub mod common;
#[cfg(feature = "x86_64")]
pub mod driver;
pub mod future;
pub mod utility;

pub use zcene_core as core;

#[cfg(not(any(feature = "hosted", test)))]
#[global_allocator]
static GLOBAL_ALLOCATOR: common::memory::allocator::PanicHeapAllocator =
    common::memory::allocator::PanicHeapAllocator;

#[cfg(feature = "x86_64")]
pub mod r#extern {
    pub use bootloader_api;
}

use alloc::sync::Arc;
use ::core::cell::SyncUnsafeCell;
use ::core::mem::MaybeUninit;

use crate::future::runtime::FutureRuntimeHandler;

pub static ACTOR_ROOT_ENVIRONMENT: SyncUnsafeCell<
    MaybeUninit<
        Arc<
            crate::actor::ActorRootEnvironment<
                FutureRuntimeHandler,
                crate::platform::ActivePlatform,
            >,
            crate::common::memory::allocator::ErasedAllocator,
        >,
    >,
> = SyncUnsafeCell::new(MaybeUninit::zeroed());

#[macro_export]
macro_rules! define_system {
    ($root_actor:expr) => {
        $crate::platform::platform_panic_handler!();
        $crate::platform::platform_entry_point!($root_actor);
    };
}
