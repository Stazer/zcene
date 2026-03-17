#![feature(abi_x86_interrupt)]
#![feature(adt_const_params)]
#![feature(allocator_api)]
#![feature(arbitrary_self_types)]
#![feature(box_as_ptr)]
#![feature(lazy_type_alias)]
#![feature(new_range_api)]
#![feature(non_lifetime_binders)]
#![feature(option_zip)]
#![feature(step_trait)]
#![feature(stmt_expr_attributes)]
#![feature(sync_unsafe_cell)]
#![feature(trait_alias)]
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
pub mod architecture;
pub mod common;
pub mod driver;
pub mod future;
pub mod kernel;
pub mod memory;
pub mod synchronization;
pub mod time;
pub mod utility;

pub mod r#extern {
    pub use bootloader_api;
}

use alloc::alloc::Global;
use alloc::sync::Arc;
use core::cell::SyncUnsafeCell;
use core::mem::MaybeUninit;

use crate::future::runtime::FutureRuntimeHandler;

pub static ACTOR_ROOT_ENVIRONMENT: SyncUnsafeCell<MaybeUninit<Arc<crate::actor::ActorRootEnvironment<FutureRuntimeHandler>, Global>>> =
    SyncUnsafeCell::new(MaybeUninit::zeroed());

#[macro_export]
macro_rules! define_system {
    ($root_actor:expr) => {
        static BOOTLOADER_CONFIG: ::zcene_bare_metal::r#extern::bootloader_api::BootloaderConfig = {
            let mut config =
                ::zcene_bare_metal::r#extern::bootloader_api::BootloaderConfig::new_default();
            config.kernel_stack_size = 4 * 4096;
            config.mappings.physical_memory =
                Some(::zcene_bare_metal::r#extern::bootloader_api::config::Mapping::Dynamic);

            config
        };

        fn entry_point(
            boot_info: &'static mut ::zcene_bare_metal::r#extern::bootloader_api::BootInfo,
        ) -> ! {
            use ::zcene_bare_metal::actor::ActorRootEnvironment;
            use ::zcene_bare_metal::ACTOR_ROOT_ENVIRONMENT;
            use ::zcene_core::actor::ActorEnvironmentSpawn;

            let environment = ActorRootEnvironment::root(
                || ::zcene_bare_metal::future::runtime::FutureRuntimeHandler::default(),
                boot_info,
            );

            unsafe {
                ACTOR_ROOT_ENVIRONMENT
                    .get()
                    .as_mut()
                    .unwrap()
                    .write(environment.clone());
            }

            let address = environment
                .spawn($root_actor)
                .unwrap();

            environment.enter();

            loop {}
        }

        ::zcene_bare_metal::r#extern::bootloader_api::entry_point!(
            entry_point,
            config = &BOOTLOADER_CONFIG
        );
    };
}
