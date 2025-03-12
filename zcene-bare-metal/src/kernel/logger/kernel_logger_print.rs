#[macro_export]
macro_rules! __print {
    ($format_string:expr) => {
        {
            use ::core::fmt::Write;

            let _ = $crate::kernel::Kernel::get()
                .logger()
                .writer(|w| write!(w, $format_string));
        }
    };
    ($format_string:expr, $($expressions:expr),+ ) => {
        {
            use ::core::fmt::Write;

            let _ = $crate::kernel::Kernel::get()
                .logger()
                .writer(|w| write!(w, $format_string, $($expressions),+));
        }
    };
}

pub use __print as print;
