use crate::common::memory::FrameIdentifier;
use crate::common::memory::address::{MemoryAddress, MemoryAddressPerspective};
use ztd::{Constructor, Method};

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Constructor, Debug, Method)]
#[Method(accessors)]
pub struct FrameIdentifierMemoryAddressTranslator {
    frame_size: usize,
}

impl FrameIdentifierMemoryAddressTranslator {
    pub fn translate_memory_address<P>(&self, memory_address: MemoryAddress<P>) -> FrameIdentifier
    where
        P: MemoryAddressPerspective,
    {
        FrameIdentifier::new(memory_address.as_usize() / self.frame_size)
    }

    pub fn translate_frame_identifier<P>(
        &self,
        frame_identifier: FrameIdentifier,
    ) -> MemoryAddress<P>
    where
        P: MemoryAddressPerspective,
    {
        MemoryAddress::new(frame_identifier.as_usize() * self.frame_size)
    }
}
