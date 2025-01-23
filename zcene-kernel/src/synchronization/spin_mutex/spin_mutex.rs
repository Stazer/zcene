use spin::Mutex;
use crate::synchronization::SpinMutexGuard;

////////////////////////////////////////////////////////////////////////////////////////////////////

pub struct SpinMutex<T>(Mutex<T>);

impl<T> Default for SpinMutex<T>
where T: Default
{
    fn default() -> Self {
        Self::new(T::default())
    }
}

impl<T> SpinMutex<T> {
    pub const fn new(v: T) -> Self {
        Self(Mutex::new(v))
    }

    pub fn lock(&self) -> SpinMutexGuard<'_, T> {
        SpinMutexGuard::new(self.0.lock())
    }
}
