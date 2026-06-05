mod platform;
pub use platform::*;

#[cfg(target_arch = "x86_64")]
pub mod x86_64;
