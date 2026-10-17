use crate::common::bits::{BitFieldEndianess, BitFieldLeftToRight, BitFieldSignificance};
use core::marker::PhantomData;
use core::sync::atomic::{AtomicU8, Ordering};

////////////////////////////////////////////////////////////////////////////////////////////////////

/// Lock-free counterpart of [`crate::common::bits::BitField`]: every operation here is a single
/// atomic RMW on the byte the bit lives in, so it is race-free under concurrent access from
/// multiple cores without any `&mut self`/lock. It is a separate type rather than another impl
/// block on `BitField` itself because Rust's coherence rules don't allow two inherent impls of
/// the same type to define methods of the same name, even under mutually exclusive trait bounds
/// on the generic parameter (nothing stops a hypothetical `T` from satisfying both `AsRef<[u8]>`
/// and `AsRef<[AtomicU8]>` as far as the compiler is concerned).
#[repr(transparent)]
pub struct AtomicBitField<R, S, T>
where
    R: BitFieldSignificance,
    S: BitFieldEndianess,
{
    significance: PhantomData<R>,
    endianess: PhantomData<S>,
    value: T,
}

impl<R, S, T> AtomicBitField<R, S, T>
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

impl<T> AtomicBitField<BitFieldLeftToRight, BitFieldLeftToRight, T> {
    #[inline]
    fn offset(position: usize) -> u8 {
        (position % (u8::BITS as usize)) as u8
    }
}

impl<T> AtomicBitField<BitFieldLeftToRight, BitFieldLeftToRight, T>
where
    T: AsRef<[AtomicU8]>,
{
    #[inline]
    pub fn len(&self) -> usize {
        self.value.as_ref().len() * (u8::BITS as usize)
    }

    #[inline]
    fn index(&self, position: usize) -> usize {
        position / (u8::BITS as usize) % self.value.as_ref().len()
    }

    #[inline]
    pub fn is_enabled(&self, position: usize) -> bool {
        let index = self.index(position);
        let mask = 0b1000_0000u8 >> Self::offset(position);

        self.value.as_ref()[index].load(Ordering::Acquire) & mask != 0
    }

    #[inline]
    pub fn is_disabled(&self, position: usize) -> bool {
        !self.is_enabled(position)
    }

    #[inline]
    pub fn enable(&self, position: usize) {
        let index = self.index(position);
        let mask = 0b1000_0000u8 >> Self::offset(position);

        self.value.as_ref()[index].fetch_or(mask, Ordering::AcqRel);
    }

    /// Like `enable`, but returns whether this call was the one that actually flipped the bit
    /// from disabled to enabled (`true`), as opposed to it already being enabled (`false`).
    #[inline]
    pub fn exclusive_enable(&self, position: usize) -> bool {
        let index = self.index(position);
        let mask = 0b1000_0000u8 >> Self::offset(position);

        self.value.as_ref()[index].fetch_or(mask, Ordering::AcqRel) & mask == 0
    }

    #[inline]
    pub fn disable(&self, position: usize) {
        let index = self.index(position);
        let mask = 0b1000_0000u8 >> Self::offset(position);

        self.value.as_ref()[index].fetch_and(!mask, Ordering::Release);
    }

    /// Like `disable`, but returns whether the bit was actually enabled beforehand (`true`), as
    /// opposed to it already being disabled (`false`) — the latter signals a double free.
    #[inline]
    pub fn exclusive_disable(&self, position: usize) -> bool {
        let index = self.index(position);
        let mask = 0b1000_0000u8 >> Self::offset(position);

        self.value.as_ref()[index].fetch_and(!mask, Ordering::Release) & mask != 0
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////

#[cfg(test)]
fn test_field(
    data: &[AtomicU8],
) -> AtomicBitField<BitFieldLeftToRight, BitFieldLeftToRight, &[AtomicU8]> {
    AtomicBitField::new(data)
}

#[test]
#[cfg(test)]
fn enable_and_disable() {
    let data: [AtomicU8; 2] = [AtomicU8::new(0), AtomicU8::new(0)];
    let field = test_field(&data);

    field.enable(0);
    field.enable(5);
    field.enable(7);
    field.enable(11);

    assert!(field.is_enabled(0));
    assert!(field.is_disabled(1));
    assert!(field.is_enabled(5));
    assert!(field.is_enabled(7));
    assert!(field.is_enabled(11));

    field.disable(5);

    assert!(field.is_disabled(5));
    assert!(field.is_enabled(7));
}

#[test]
#[cfg(test)]
fn exclusive_enable_reports_winner() {
    let data: [AtomicU8; 1] = [AtomicU8::new(0)];
    let field = test_field(&data);

    assert!(field.exclusive_enable(3));
    assert!(!field.exclusive_enable(3));
    assert!(field.is_enabled(3));
}

#[test]
#[cfg(test)]
fn exclusive_disable_reports_double_free() {
    let data: [AtomicU8; 1] = [AtomicU8::new(0)];
    let field = test_field(&data);

    field.enable(2);

    assert!(field.exclusive_disable(2));
    assert!(!field.exclusive_disable(2));
    assert!(field.is_disabled(2));
}
