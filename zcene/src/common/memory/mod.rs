pub mod address;
pub mod allocator;
pub mod region;
pub mod resource;

mod frame_identifier;
mod frame_identifier_memory_address_translator;

pub use frame_identifier::*;
pub use frame_identifier_memory_address_translator::*;
