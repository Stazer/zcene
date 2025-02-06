use crate::kernel::logger::KernelLogger;
use ztd::Constructor;
use core::fmt::{Result, Write};

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Constructor)]
pub struct KernelLoggerWriter<'a> {
    logger: &'a KernelLogger,

}

impl<'a> Write for KernelLoggerWriter<'a> {
    fn write_str(&mut self, s: &str) -> Result {
        self.logger.write(s);

        Ok(())
    }
}
