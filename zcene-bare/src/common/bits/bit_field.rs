use crate::common::bits::{BitFieldEndianess, BitFieldLeftToRight, BitFieldSignificance};
use core::marker::PhantomData;

////////////////////////////////////////////////////////////////////////////////////////////////////

#[repr(transparent)]
pub struct BitField<R, S, T>
where
    R: BitFieldSignificance,
    S: BitFieldEndianess,
{
    significance: PhantomData<R>,
    endianess: PhantomData<S>,
    value: T,
}

impl<R, S, T> BitField<R, S, T>
where
    R: BitFieldSignificance,
    S: BitFieldEndianess,
{
    pub fn new(value: T) -> Self {
        Self {
            significance: PhantomData::<R>,
            endianess: PhantomData::<S>,
            value,
        }
    }
}

impl<T> BitField<BitFieldLeftToRight, BitFieldLeftToRight, T> {
    #[inline]
    fn offset(position: usize) -> u8 {
        (position % (u8::BITS as usize)) as u8
    }
}

impl<T> BitField<BitFieldLeftToRight, BitFieldLeftToRight, T>
where
    T: AsRef<[u8]>,
{
    #[inline]
    pub fn len(&self) -> usize {
        self.value.as_ref().len() * (u8::BITS as usize)
    }

    #[inline]
    pub fn is_enabled(&self, position: usize) -> bool {
        self.value.as_ref()[self.index(position)] & 0b1000_0000u8 >> Self::offset(position) != 0
    }

    #[inline]
    pub fn is_disabled(&self, position: usize) -> bool {
        !self.is_enabled(position)
    }

    #[inline]
    fn index(&self, position: usize) -> usize {
        position / (u8::BITS as usize) % self.value.as_ref().len()
    }
}

impl<T> BitField<BitFieldLeftToRight, BitFieldLeftToRight, T>
where
    T: AsRef<[u8]> + AsMut<[u8]>,
{
    #[inline]
    pub fn enable(&mut self, position: usize) {
        let index = self.index(position);

        self.value.as_mut()[index] |= 0b1000_0000u8 >> Self::offset(position);
    }

    #[inline]
    pub fn disable(&mut self, position: usize) {
        let index = self.index(position);

        self.value.as_mut()[index] &= !(0b1000_0000u8 >> Self::offset(position));
    }

    #[inline]
    pub fn toggle(&mut self, position: usize) {
        let index = self.index(position);

        self.value.as_mut()[index] ^= 0b1000_0000u8 >> Self::offset(position);
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////

#[test]
#[cfg(test)]
fn is_enabled() {
    let data: [u8; 2] = [0b0000_1111u8, 0b1011_1011u8];
    let field = BitField::<BitFieldLeftToRight, BitFieldLeftToRight, _>::new(&data);

    assert!(!field.is_enabled(0));
    assert!(!field.is_enabled(1));
    assert!(!field.is_enabled(2));
    assert!(!field.is_enabled(3));
    assert!(field.is_enabled(4));
    assert!(field.is_enabled(5));
    assert!(field.is_enabled(6));
    assert!(field.is_enabled(7));

    assert!(field.is_enabled(8));
    assert!(!field.is_enabled(9));
    assert!(field.is_enabled(10));
    assert!(field.is_enabled(11));
    assert!(field.is_enabled(12));
    assert!(!field.is_enabled(13));
    assert!(field.is_enabled(14));
    assert!(field.is_enabled(15));
}

#[test]
#[cfg(test)]
fn is_disabled() {
    let data: [u8; 2] = [0b0000_1111u8, 0b1011_1011u8];
    let field = BitField::<BitFieldLeftToRight, BitFieldLeftToRight, _>::new(&data);

    assert!(field.is_disabled(0));
    assert!(field.is_disabled(1));
    assert!(field.is_disabled(2));
    assert!(field.is_disabled(3));
    assert!(!field.is_disabled(4));
    assert!(!field.is_disabled(5));
    assert!(!field.is_disabled(6));
    assert!(!field.is_disabled(7));

    assert!(!field.is_disabled(8));
    assert!(field.is_disabled(9));
    assert!(!field.is_disabled(10));
    assert!(!field.is_disabled(11));
    assert!(!field.is_disabled(12));
    assert!(field.is_disabled(13));
    assert!(!field.is_disabled(14));
    assert!(!field.is_disabled(15));
}

#[test]
#[cfg(test)]
fn enable() {
    let mut data: [u8; 2] = [0b0000_0000u8, 0b0000_0000u8];
    let mut field = BitField::<BitFieldLeftToRight, BitFieldLeftToRight, _>::new(&mut data);

    field.enable(0);
    field.enable(5);
    field.enable(7);
    field.enable(11);
    field.enable(12);
    field.enable(15);

    assert!(field.is_enabled(0));
    assert!(field.is_disabled(1));
    assert!(field.is_disabled(2));
    assert!(field.is_disabled(3));
    assert!(field.is_disabled(4));
    assert!(field.is_enabled(5));
    assert!(field.is_disabled(6));
    assert!(field.is_enabled(7));

    assert!(field.is_disabled(8));
    assert!(field.is_disabled(9));
    assert!(field.is_disabled(10));
    assert!(field.is_enabled(11));
    assert!(field.is_enabled(12));
    assert!(field.is_disabled(13));
    assert!(field.is_disabled(14));
    assert!(field.is_enabled(15));
}

#[test]
#[cfg(test)]
fn disable() {
    let mut data: [u8; 2] = [0b1111_1111u8, 0b1111_1111u8];
    let mut field = BitField::<BitFieldLeftToRight, BitFieldLeftToRight, _>::new(&mut data);

    field.disable(0);
    field.disable(5);
    field.disable(7);
    field.disable(11);
    field.disable(12);
    field.disable(15);

    assert!(field.is_disabled(0));
    assert!(field.is_enabled(1));
    assert!(field.is_enabled(2));
    assert!(field.is_enabled(3));
    assert!(field.is_enabled(4));
    assert!(field.is_disabled(5));
    assert!(field.is_enabled(6));
    assert!(field.is_disabled(7));

    assert!(field.is_enabled(8));
    assert!(field.is_enabled(9));
    assert!(field.is_enabled(10));
    assert!(field.is_disabled(11));
    assert!(field.is_disabled(12));
    assert!(field.is_enabled(13));
    assert!(field.is_enabled(14));
    assert!(field.is_disabled(15));
}

#[test]
#[cfg(test)]
fn toggle() {
    let mut data: [u8; 2] = [0b0000_1111u8, 0b1011_1011u8];
    let mut field = BitField::<BitFieldLeftToRight, BitFieldLeftToRight, _>::new(&mut data);

    field.toggle(0);
    field.toggle(5);
    field.toggle(7);
    field.toggle(11);
    field.toggle(12);
    field.toggle(15);

    assert!(field.is_enabled(0));
    assert!(field.is_disabled(1));
    assert!(field.is_disabled(2));
    assert!(field.is_disabled(3));
    assert!(field.is_enabled(4));
    assert!(field.is_disabled(5));
    assert!(field.is_enabled(6));
    assert!(field.is_disabled(7));

    assert!(field.is_enabled(8));
    assert!(field.is_disabled(9));
    assert!(field.is_enabled(10));
    assert!(field.is_disabled(11));
    assert!(field.is_disabled(12));
    assert!(field.is_disabled(13));
    assert!(field.is_enabled(14));
    assert!(field.is_disabled(15));
}
