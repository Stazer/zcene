use crate::common::memory::address::MemoryAddressPerspective;
use crate::common::memory::allocator::FlatBitmapFrameManager;

////////////////////////////////////////////////////////////////////////////////////////////////////

pub type FrameManager<'a, P>
    = FlatBitmapFrameManager<'a, P>
where
    P: MemoryAddressPerspective;
