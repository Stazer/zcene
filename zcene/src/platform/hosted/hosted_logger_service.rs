use core::fmt::{self, Write};

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Default)]
pub struct HostedLoggerService;

impl HostedLoggerService {
    pub fn new() -> Self {
        Self
    }

    pub fn writer(&self) -> impl Write + '_ {
        struct Writer;

        impl Write for Writer {
            fn write_str(&mut self, string: &str) -> fmt::Result {
                use std::io::Write as _;

                print!("{}", string);
                let _ = std::io::stdout().flush();

                Ok(())
            }
        }

        Writer
    }
}
