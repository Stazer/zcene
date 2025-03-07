use crate::kernel::logger::KernelLoggerWriter;
use bootloader_x86_64_common::framebuffer::FrameBufferWriter;
use bootloader_x86_64_common::serial::SerialPort;
use core::fmt::{Result, Write};
use x86_64::instructions::interrupts::without_interrupts;
use crate::synchronization::Mutex;

////////////////////////////////////////////////////////////////////////////////////////////////////

pub struct KernelLogger {
    frame_buffer: Mutex<Option<FrameBufferWriter>>,
    serial_port: Mutex<Option<SerialPort>>,
}

impl KernelLogger {
    pub fn new(frame_buffer: Option<FrameBufferWriter>, serial_port: Option<SerialPort>) -> Self {
        Self {
            frame_buffer: Mutex::new(frame_buffer),
            serial_port: Mutex::new(serial_port),
        }
    }

    pub fn write(&self, string: &str) {
        without_interrupts(|| {
            if let Some(frame_buffer) = self.frame_buffer.lock().as_mut() {
                let _ = write!(frame_buffer, "{}", string);
            }

            if let Some(serial_port) = self.serial_port.lock().as_mut() {
                let _ = write!(serial_port, "{}", string);
            }
        })
    }

    pub fn writer<F>(&self, function: F) -> Result
    where
        F: FnOnce(&mut KernelLoggerWriter<'_>) -> Result,
    {
        function(&mut KernelLoggerWriter::new(self))
    }
}
