use crate::kernel::Kernel;
use core::fmt::Write;

pub struct Mutex<T>(spin::Mutex<T>);

impl<T> Default for Mutex<T>
where T: Default
{
    fn default() -> Self {
        Self::new(T::default())
    }
}

impl<T> Mutex<T> {
    pub fn new(v: T) -> Self {
        Self(spin::Mutex::new(v))
    }

    pub fn lock(&self) -> MutexGuard<'_, T> {
        MutexGuard(self.0.lock())
    }
}

pub struct MutexGuard<'a, T>(spin::MutexGuard<'a, T>);

impl<'a, T> core::ops::Deref for MutexGuard<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.0.deref()
    }
}

impl<'a, T> core::ops::DerefMut for MutexGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0.deref_mut()
    }
}

impl<'a, T> Drop for MutexGuard<'a, T> {
    fn drop(&mut self) {

    }
}
