#[macro_export]
macro_rules! __println {
    ($format_string:expr) => {
        {
            $crate::kernel::logger::print!(concat!($format_string, "\n"));
        }
    };
    ($format_string:expr, $($expressions:expr),+ ) => {
        {
            $crate::kernel::logger::print!(concat!($format_string, "\n"), $($expressions),+);
        }
    };
}

pub use __println as println;
