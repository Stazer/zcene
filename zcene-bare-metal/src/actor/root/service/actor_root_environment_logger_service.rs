use crate::synchronization::Mutex;
use bootloader_x86_64_common::framebuffer::FrameBufferWriter;
use bootloader_x86_64_common::serial::SerialPort;
use x86_64::instructions::interrupts::without_interrupts;
use core::fmt::{self, Write};

////////////////////////////////////////////////////////////////////////////////////////////////////

pub struct ActorRootEnvironmentLoggerService {
    frame_buffer: Mutex<Option<FrameBufferWriter>>,
    serial_port: Mutex<Option<SerialPort>>,
}

impl ActorRootEnvironmentLoggerService {
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

    pub fn writer<'a>(&'a self) -> impl Write + 'a {
        pub struct Writer<'b> {
            service: &'b ActorRootEnvironmentLoggerService
        }

        impl<'b> Write for Writer<'b> {
            fn write_str(&mut self, string: &str) -> Result<(), core::fmt::Error> {
                self.service.write(string);

                Ok(())
            }
        }

        Writer {
            service: self
        }
    }
}
