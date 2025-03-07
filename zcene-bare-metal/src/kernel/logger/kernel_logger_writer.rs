use crate::kernel::logger::KernelLogger;
use core::fmt::{Result, Write};
use ztd::Constructor;

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
