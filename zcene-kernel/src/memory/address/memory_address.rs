use crate::memory::address::{
    MemoryAddressPerspective,
};
use core::fmt::{self, Debug};
use core::marker::PhantomData;

////////////////////////////////////////////////////////////////////////////////////////////////////

#[repr(transparent)]
#[derive(Copy, Clone)]
pub struct MemoryAddress<P>
where
    P: MemoryAddressPerspective,
{
    perspective: PhantomData<P>,
    value: usize,
}

impl<P> Debug for MemoryAddress<P>
where
    P: MemoryAddressPerspective,
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "0x{:X}", self.value)
    }
}

impl<P> MemoryAddress<P>
where
    P: MemoryAddressPerspective,
{
    pub fn new(value: usize) -> Self {
        Self {
            perspective: PhantomData::<P>,
            value,
        }
    }

    pub fn as_usize(&self) -> usize {
        self.value
    }

    #[cfg(target_pointer_width = "64")]
    pub fn as_u64(&self) -> u64 {
        self.value as _
    }
}

impl<P> From<usize> for MemoryAddress<P>
where
    P: MemoryAddressPerspective,
{
    fn from(value: usize) -> Self {
        Self {
            perspective: PhantomData::<P>,
            value,
        }
    }
}

#[cfg(target_pointer_width = "64")]
impl<P> From<u64> for MemoryAddress<P>
where
    P: MemoryAddressPerspective,
{
    fn from(value: u64) -> Self {
        Self {
            perspective: PhantomData::<P>,
            value: value as _,
        }
    }
}
