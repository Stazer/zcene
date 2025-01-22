pub mod actor;
pub mod future;
pub mod x86;
mod mutex;

pub use mutex::*;

macro_rules! println {
    ($format_string:expr) => {
        let _ = Kernel::get()
            .logger()
            .writer(|w| writeln!(w, $format_string));
    };
    ($format_string:expr, $($expressions:expr),+ ) => {
        let _ Kernel::get()
            .logger()
            .writer(|w| writeln!(w, $format_string, $($expressions),+));
    };
}

pub(crate) use println;
