#[inline]
pub fn alignment_length<T>(value: usize, alignment: T) -> usize
where
    T: Into<usize>,
{
    let mask = alignment.into() - 1;
    if value & mask != 0 {
        return ((value | mask) + 1) - value;
    }

    0
}
