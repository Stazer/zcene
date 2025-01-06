use core::num::NonZero;
use ztd::{Constructor, Method};

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Constructor, Debug, PartialEq, Method)]
#[Method(accessors)]
pub struct MemoryResourceAllocationRegion {
    start: usize,
    header_length: usize,
    alignment_length: usize,
    data_length: NonZero<usize>,
    padding_length: usize,
}

impl MemoryResourceAllocationRegion {
    pub fn total_length(&self) -> NonZero<usize> {
        self.data_length
            .checked_add(self.header_length + self.alignment_length + self.padding_length)
            .unwrap()
    }

    pub fn end(&self) -> NonZero<usize> {
        self.total_length().checked_add(self.start).unwrap()
    }

    pub fn data_start(&self) -> usize {
        self.start + self.header_length + self.alignment_length
    }

    pub fn data_end(&self) -> NonZero<usize> {
        self.data_length.checked_add(self.data_start()).unwrap()
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////

#[test]
#[cfg(test)]
fn total_length() {
    assert_eq!(
        NonZero::new(15),
        NonZero::new(4).map(|data_length| {
            MemoryResourceAllocationRegion::new(0, 1, 2, data_length, 8).total_length()
        }),
    );
}

#[test]
#[cfg(test)]
fn end() {
    assert_eq!(
        NonZero::new(20),
        NonZero::new(4).map(|data_length| {
            MemoryResourceAllocationRegion::new(5, 1, 2, data_length, 8).end()
        }),
    );
}

#[test]
#[cfg(test)]
fn data_start() {
    assert_eq!(
        Some(8),
        NonZero::new(4).map(|data_length| {
            MemoryResourceAllocationRegion::new(5, 1, 2, data_length, 8).data_start()
        }),
    );
}

#[test]
#[cfg(test)]
fn data_end() {
    assert_eq!(
        NonZero::new(12),
        NonZero::new(4).map(|data_length| {
            MemoryResourceAllocationRegion::new(5, 1, 2, data_length, 8).data_end()
        }),
    );
}
