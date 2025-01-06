use crate::common::memory::MemoryAddressPerspective;
use crate::memory::frame::FlatBitmapFrameManager;

////////////////////////////////////////////////////////////////////////////////////////////////////

pub type FrameManager<'a, T>
    = FlatBitmapFrameManager<'a, T>
where
    T: MemoryAddressPerspective;
