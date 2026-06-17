pub(crate) use core::cell;
pub(crate) use core::sync::atomic;

#[inline]
pub(crate) fn busy_wait() {
    core::hint::spin_loop();
}

pub(crate) mod prelude {
    use super::{atomic, cell};

    pub(crate) trait UnsafeCellExt {
        type Value;

        fn with_mut<R, F>(&self, f: F) -> R
        where
            F: FnOnce(*mut Self::Value) -> R;
    }

    impl<T> UnsafeCellExt for cell::UnsafeCell<T> {
        type Value = T;

        fn with_mut<R, F>(&self, f: F) -> R
        where
            F: FnOnce(*mut Self::Value) -> R,
        {
            f(self.get())
        }
    }

    pub(crate) trait AtomicExt {
        type Value;

        fn with_mut<R, F>(&mut self, f: F) -> R
        where
            F: FnOnce(&mut Self::Value) -> R;
    }

    impl AtomicExt for atomic::AtomicUsize {
        type Value = usize;

        fn with_mut<R, F>(&mut self, f: F) -> R
        where
            F: FnOnce(&mut Self::Value) -> R,
        {
            f(self.get_mut())
        }
    }

    impl<T> AtomicExt for atomic::AtomicPtr<T> {
        type Value = *mut T;

        fn with_mut<R, F>(&mut self, f: F) -> R
        where
            F: FnOnce(&mut Self::Value) -> R,
        {
            f(self.get_mut())
        }
    }
}
