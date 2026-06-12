#![feature(abi_x86_interrupt)]
#![feature(adt_const_params)]
#![feature(allocator_api)]
#![feature(arbitrary_self_types)]
#![feature(box_as_ptr)]
#![feature(btreemap_alloc)]
#![feature(lazy_type_alias)]
#![feature(non_lifetime_binders)]
#![feature(step_trait)]
#![feature(stmt_expr_attributes)]
#![feature(sync_unsafe_cell)]
#![feature(unsized_const_params)]
#![no_std]

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
pub mod driver;
pub mod future;
pub mod utility;

#[global_allocator]
static GLOBAL_ALLOCATOR: common::memory::allocator::PanicHeapAllocator =
    common::memory::allocator::PanicHeapAllocator;

pub mod r#extern {
    pub use bootloader_api;
}

use alloc::sync::Arc;
use core::cell::SyncUnsafeCell;
use core::mem::MaybeUninit;

use crate::future::runtime::FutureRuntimeHandler;

pub static ACTOR_ROOT_ENVIRONMENT: SyncUnsafeCell<
    MaybeUninit<
        Arc<
            crate::actor::ActorRootEnvironment<
                FutureRuntimeHandler,
                crate::platform::x86_64::X86_64Platform,
            >,
            crate::common::memory::allocator::ErasedAllocator,
        >,
    >,
> = SyncUnsafeCell::new(MaybeUninit::zeroed());

#[macro_export]
macro_rules! define_system {
    ($root_actor:expr) => {
        static BOOTLOADER_CONFIG: $crate::r#extern::bootloader_api::BootloaderConfig = {
            let mut config =
                $crate::r#extern::bootloader_api::BootloaderConfig::new_default();
            config.kernel_stack_size = 4 * 4096;
            config.mappings.physical_memory =
                Some($crate::r#extern::bootloader_api::config::Mapping::Dynamic);

            config
        };

        #[panic_handler]
        fn panic_handler(panic_info: &::core::panic::PanicInfo) -> ! {
            use core::fmt::Write;

            let _ = write!(
                $crate::actor::ActorRootEnvironment::get().logger(),
                "Panic: {:?}",
                panic_info,
            );

            loop {}
        }

        fn entry_point(
            boot_info: &'static mut $crate::r#extern::bootloader_api::BootInfo,
        ) -> ! {
            use $crate::ACTOR_ROOT_ENVIRONMENT;
            use $crate::actor::ActorRootEnvironment;
            use $crate::future::runtime::FutureRuntimeHandler;

            let environment =
                ActorRootEnvironment::root(FutureRuntimeHandler::new, boot_info);

            unsafe {
                ACTOR_ROOT_ENVIRONMENT
                    .get()
                    .as_mut()
                    .unwrap()
                    .write(environment.clone());
            }

            let address = environment.spawn($root_actor).unwrap();

            environment.enter().unwrap();

            loop {}
        }

        $crate::r#extern::bootloader_api::entry_point!(
            entry_point,
            config = &BOOTLOADER_CONFIG
        );
    };
}
