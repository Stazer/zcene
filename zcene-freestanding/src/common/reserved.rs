#[repr(C)]
pub struct Reserved<const N: usize, T> {
    _value: [T; N],
}
