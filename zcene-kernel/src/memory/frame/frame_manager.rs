use crate::memory::address::MemoryAddressPerspective;
use crate::memory::frame::FlatBitmapFrameManager;

////////////////////////////////////////////////////////////////////////////////////////////////////

pub type FrameManager<'a, P>
    = FlatBitmapFrameManager<'a, P>
where
    P: MemoryAddressPerspective;
