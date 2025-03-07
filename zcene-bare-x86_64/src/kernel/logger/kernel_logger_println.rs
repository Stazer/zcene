macro_rules! println {
    ($format_string:expr) => {
        {
            crate::kernel::logger::print!(concat!($format_string, "\n"));
        }
    };
    ($format_string:expr, $($expressions:expr),+ ) => {
        {
            crate::kernel::logger::print!(concat!($format_string, "\n"), $($expressions),+);
        }
    };
}

pub(crate) use println;
