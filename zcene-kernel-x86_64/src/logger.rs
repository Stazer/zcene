use bootloader_api::info::FrameBuffer;
use bootloader_api::BootInfo;
use bootloader_x86_64_common::framebuffer::FrameBufferWriter;
use core::fmt::{self, Write};
use log::{Log, Metadata, Record};
use x86_64::instructions::interrupts::without_interrupts;
use bootloader_x86_64_common::serial::SerialPort;
use zcene_kernel::synchronization::Mutex;

////////////////////////////////////////////////////////////////////////////////////////////////////

pub struct Logger(Mutex<Option<FrameBufferWriter>>, Mutex<Option<SerialPort>>);

impl Log for Logger {
    fn enabled(&self, _metadata: &Metadata<'_>) -> bool {
        true
    }

    fn log(&self, record: &Record<'_>) {
        without_interrupts(|| {
            if let Some(writer) = self.0.lock().as_mut() {
                let _ = write!(writer, "[{}] {}\n", record.level(), record.args(),);
            }
        })
    }

    fn flush(&self) {}
}

use bootloader_api::info::Optional;

impl Logger {
    pub const fn new() -> Self {
        Self(Mutex::new(None), Mutex::new(None))
    }

    pub fn attach_boot_info(&self, boot_info: &'static mut BootInfo) {
        if let Optional::Some(ref mut framebuffer) = boot_info.framebuffer {
            self.attach_frame_buffer(framebuffer);
        }

        unsafe {
            *self.1.lock() = Some(SerialPort::init());
        }
    }

    pub fn attach_frame_buffer<'a>(&self, frame_buffer: &'static mut FrameBuffer) {
        let info = frame_buffer.info().clone();

        without_interrupts(|| {
            *self.0.lock() = Some(FrameBufferWriter::new(frame_buffer.buffer_mut(), info));
        })
    }

    pub fn write(&self, string: &str) {
        without_interrupts(|| {
            if let Some(writer) = self.0.lock().as_mut() {
                let _ = write!(writer, "{}", string,);
            }

            if let Some(writer) = self.1.lock().as_mut() {
                let _ = write!(writer, "{}", string);
            }
        })
    }

    pub fn writer<F>(&self, function: F) -> Result<(), fmt::Error>
    where
        F: FnOnce(&mut InnerLogger<'_>) -> Result<(), fmt::Error>,
    {
        without_interrupts(|| {
            function(&mut InnerLogger(self));

            Ok(())
        })
    }
}

pub struct InnerLogger<'a>(&'a Logger);

impl<'a> core::fmt::Write for InnerLogger<'a> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        without_interrupts(|| {
            self.0.write(s);
        });

        Ok(())
    }
}
