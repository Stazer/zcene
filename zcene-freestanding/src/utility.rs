#[macro_export]
macro_rules! __print {
    ($format_string:expr) => {
        {
            use ::core::fmt::Write;

            let _ = $crate::actor::ActorRootEnvironment::get()
                .logger
                .writer(|w| write!(w, $format_string));

            let _ = write!(
                $crate::actor::ActorRootEnvironment::get().logger(),
                $format_string
            );
        }
    };
    ($format_string:expr, $($expressions:expr),+ ) => {
        {
            use ::core::fmt::Write;

            let _ = write!(
                $crate::actor::ActorRootEnvironment::get().logger(),
                $format_string,
                $($expressions),+
            );
        }
    };
}

pub use __print as print;

#[macro_export]
macro_rules! __println {
    ($format_string:expr) => {
        {
            $crate::utility::print!(concat!($format_string, "\n"));
        }
    };
    ($format_string:expr, $($expressions:expr),+ ) => {
        {
            $crate::utility::print!(concat!($format_string, "\n"), $($expressions),+);
        }
    };
}

pub use __println as println;
