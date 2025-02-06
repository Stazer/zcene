macro_rules! println {
    ($format_string:expr) => {
        {
            use ::core::fmt::Write;

            let _ = crate::kernel::Kernel::get()
                .logger()
                .writer(|w| writeln!(w, $format_string));
        }
    };
    ($format_string:expr, $($expressions:expr),+ ) => {
        {
            use ::core::fmt::Write;

            let _ = crate::kernel::Kernel::get()
                .logger()
                .writer(|w| writeln!(w, $format_string, $($expressions),+));
        }
    };
}

pub(crate) use println;

macro_rules! print {
    ($format_string:expr) => {
        {
            use ::core::fmt::Write;

            let _ = crate::kernel::Kernel::get()
                .logger()
                .writer(|w| writeln!(w, $format_string));
        }
    };
    ($format_string:expr, $($expressions:expr),+ ) => {
        {
            use ::core::fmt::Write;

            let _ = crate::kernel::Kernel::get()
                .logger()
                .writer(|w| writeln!(w, $format_string, $($expressions),+));
        }
    };
}

