use core::marker::PhantomData;
use crate::common::volatile::{VolatileReadWriteAccessMode, VolatileAccessMode, VolatileWritingAccessMode, VolatileReadingAccessMode};
use core::ptr::{write_volatile, read_volatile};

////////////////////////////////////////////////////////////////////////////////////////////////////

#[repr(transparent)]
pub struct Volatile<T, A = VolatileReadWriteAccessMode>(T, PhantomData<A>)
where
    A: VolatileAccessMode;

impl<T, A> Volatile<T, A>
where
    A: VolatileAccessMode
{
    pub fn new(value: T) -> Self {
        Self(value, PhantomData::<A>)
    }

    pub fn write(&mut self, value: T)
    where
        A: VolatileWritingAccessMode,
    {
        unsafe {
            write_volatile(&mut self.0, value)
        }
    }

    pub fn read(&self) -> T
    where
        A: VolatileReadingAccessMode,
    {
        unsafe {
            read_volatile(&self.0)
        }
    }

    pub fn update<F>(&mut self, function: F)
    where
        F: FnOnce(T) -> T,
        A: VolatileReadingAccessMode + VolatileWritingAccessMode,
    {
        self.write(function(self.read()))
    }
}
