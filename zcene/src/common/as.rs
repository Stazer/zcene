pub trait As<T> {
    fn r#as(&self) -> T;
}

#[cfg(target_pointer_width = "64")]
impl As<u64> for usize {
    fn r#as(&self) -> u64 {
        *self as _
    }
}

#[cfg(target_pointer_width = "64")]
impl As<usize> for u64 {
    fn r#as(&self) -> usize {
        *self as _
    }
}

#[cfg(target_pointer_width = "32")]
impl As<usize> for u32 {
    fn r#as(&self) -> usize {
        *self as _
    }
}

#[cfg(target_pointer_width = "32")]
impl As<usize> for u32 {
    fn r#as(&self) -> u32 {
        *self as _
    }
}
