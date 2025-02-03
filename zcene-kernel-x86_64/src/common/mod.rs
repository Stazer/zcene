macro_rules! println {
    ($format_string:expr) => {
        let _ = crate::kernel::Kernel::get()
            .logger()
            .writer(|w| writeln!(w, $format_string));
    };
    ($format_string:expr, $($expressions:expr),+ ) => {
        let _ = crate::kernel::Kernel::get()
            .logger()
            .writer(|w| writeln!(w, $format_string, $($expressions),+));
    };
}

pub(crate) use println;

macro_rules! print {
    ($format_string:expr) => {
        let _ = crate::kernel::Kernel::get()
            .logger()
            .writer(|w| writeln!(w, $format_string));
    };
    ($format_string:expr, $($expressions:expr),+ ) => {
        let _ = crate::kernel::Kernel::get()
            .logger()
            .writer(|w| writeln!(w, $format_string, $($expressions),+));
    };
}

pub(crate) use print;
