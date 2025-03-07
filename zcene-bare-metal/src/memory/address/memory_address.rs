use crate::memory::address::{MemoryAddressPerspective, MemoryAddressTransformer, DefaultMemoryAddressTransformer};
use core::fmt::{self, Debug};
use core::marker::PhantomData;

////////////////////////////////////////////////////////////////////////////////////////////////////

#[repr(transparent)]
#[derive(Copy, Clone)]
pub struct MemoryAddress<P, T = DefaultMemoryAddressTransformer>
where
    P: MemoryAddressPerspective,
    T: MemoryAddressTransformer,
{
    value: usize,
    perspective: PhantomData<P>,
    truncater: PhantomData<T>,
}

impl<P, T> Debug for MemoryAddress<P, T>
where
    P: MemoryAddressPerspective,
    T: MemoryAddressTransformer,
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "0x{:x}", self.value)
    }
}

impl<P, T> MemoryAddress<P, T>
where
    P: MemoryAddressPerspective,
    T: MemoryAddressTransformer,
{
    pub fn new(value: usize) -> Self {
        Self {
            value: T::transform(value),
            perspective: PhantomData::<P>,
            truncater: PhantomData::<T>,
        }
    }

    pub fn as_usize(&self) -> usize {
        self.value
    }

    #[cfg(target_pointer_width = "64")]
    pub fn as_u64(&self) -> u64 {
        self.value as _
    }

    pub fn as_pointer(&self) -> *const () {
        self.value as *const ()
    }

    pub fn as_mut_pointer(&self) -> *mut () {
        self.value as *mut ()
    }

    pub fn cast<S>(&self) -> *const S {
        self.value as *const S
    }

    pub fn cast_mut<S>(&self) -> *mut S {
        self.value as *mut S
    }
}

impl<P, T> From<usize> for MemoryAddress<P, T>
where
    P: MemoryAddressPerspective,
    T: MemoryAddressTransformer,
{
    fn from(value: usize) -> Self {
        Self {
            value,
            perspective: PhantomData::<P>,
            truncater: PhantomData::<T>,
        }
    }
}

#[cfg(target_pointer_width = "64")]
impl<P, T> From<u64> for MemoryAddress<P, T>
where
    P: MemoryAddressPerspective,
    T: MemoryAddressTransformer,
{
    fn from(value: u64) -> Self {
        Self {
            value: value as _,
            perspective: PhantomData::<P>,
            truncater: PhantomData::<T>,
        }
    }
}
