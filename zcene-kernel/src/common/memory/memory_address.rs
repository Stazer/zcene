use crate::common::memory::{
    MemoryAddressPerspective, PhysicalMemoryAddress, VirtualMemoryAddress,
};
use core::fmt::{self, Debug};
use core::marker::PhantomData;

////////////////////////////////////////////////////////////////////////////////////////////////////

#[repr(transparent)]
#[derive(Copy, Clone)]
pub struct MemoryAddress<T>
where
    T: MemoryAddressPerspective,
{
    perspective: PhantomData<T>,
    value: u64,
}

impl<T> Debug for MemoryAddress<T>
where
    T: MemoryAddressPerspective,
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "0x{:X}", self.value)
    }
}

impl<S, T> From<*const S> for MemoryAddress<T>
where
    T: MemoryAddressPerspective,
{
    fn from(pointer: *const S) -> Self {
        Self::new(pointer as u64)
    }
}

impl<S, T> From<*mut S> for MemoryAddress<T>
where
    T: MemoryAddressPerspective,
{
    fn from(pointer: *mut S) -> Self {
        Self::new(pointer as u64)
    }
}

impl<S, T> From<MemoryAddress<T>> for *mut S
where
    T: MemoryAddressPerspective,
{
    fn from(address: MemoryAddress<T>) -> Self {
        address.as_mut_ptr()
    }
}

impl<T> MemoryAddress<T>
where
    T: MemoryAddressPerspective,
{
    pub fn new(value: u64) -> Self {
        Self {
            perspective: PhantomData::<T>,
            value,
        }
    }

    pub fn as_u64(&self) -> u64 {
        self.value
    }

    pub fn as_usize(&self) -> usize {
        // TODO: Enforce at compile time
        self.value as usize
    }

    pub fn as_ptr<S>(&self) -> *const S {
        self.value as *const S
    }

    pub fn as_mut_ptr<S>(&self) -> *mut S {
        self.value as *mut S
    }
}

impl VirtualMemoryAddress {
    pub fn as_physical(&self) -> PhysicalMemoryAddress {
        PhysicalMemoryAddress::new(self.value)
    }
}

impl PhysicalMemoryAddress {
    pub fn as_virtual(&self) -> VirtualMemoryAddress {
        VirtualMemoryAddress::new(self.value)
    }
}
