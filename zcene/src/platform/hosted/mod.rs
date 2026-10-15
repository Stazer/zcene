mod hosted_logger_service;
pub use hosted_logger_service::*;

mod hosted_platform;
pub use hosted_platform::*;

////////////////////////////////////////////////////////////////////////////////////////////////////

#[macro_export]
macro_rules! hosted_panic_handler {
    () => {};
}

pub use hosted_panic_handler;

#[macro_export]
macro_rules! hosted_entry_point {
    ($root_actor:expr) => {
        fn main() {
            use $crate::ACTOR_ROOT_ENVIRONMENT;
            use $crate::actor::ActorRootEnvironment;
            use $crate::future::runtime::FutureRuntimeHandler;

            let environment = ActorRootEnvironment::root(FutureRuntimeHandler::new, ());

            unsafe {
                ACTOR_ROOT_ENVIRONMENT
                    .get()
                    .as_mut()
                    .unwrap()
                    .write(environment.clone());
            }

            let _address = environment.spawn($root_actor).unwrap();

            environment.enter().unwrap();
        }
    };
}

pub use hosted_entry_point;
