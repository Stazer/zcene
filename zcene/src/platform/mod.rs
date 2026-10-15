mod platform;
pub use platform::*;

pub mod unprivileged_event;
pub use unprivileged_event::{UnprivilegedEvent, UnprivilegedSystemCallEvent};

#[cfg(feature = "x86_64")]
pub mod x86_64;

#[cfg(feature = "hosted")]
pub mod hosted;

#[cfg(all(feature = "x86_64", feature = "hosted"))]
compile_error!("features \"x86_64\" and \"hosted\" are mutually exclusive");

#[cfg(not(any(feature = "x86_64", feature = "hosted")))]
compile_error!("exactly one platform feature must be enabled: \"x86_64\" or \"hosted\"");

#[cfg(feature = "x86_64")]
pub type ActivePlatform = x86_64::X86_64Platform;

#[cfg(feature = "hosted")]
pub type ActivePlatform = hosted::HostedPlatform;

#[cfg(feature = "x86_64")]
pub use x86_64::x86_64_entry_point as platform_entry_point;

#[cfg(feature = "hosted")]
pub use hosted::hosted_entry_point as platform_entry_point;

#[cfg(feature = "x86_64")]
pub use x86_64::x86_64_panic_handler as platform_panic_handler;

#[cfg(feature = "hosted")]
pub use hosted::hosted_panic_handler as platform_panic_handler;
