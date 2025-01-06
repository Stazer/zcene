use crate::common::alignment_length;
use crate::memory::resource::{
    MemoryResourceAllocationError, MemoryResourceAllocationRegion, MemoryResourceAllocationRequest,
    MemoryResourceDeallocationError, MemoryResourceDeallocationRequest, MemoryResourceStrategy,
};
use ztd::Method;

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Default, Debug, Method)]
#[Method(accessors)]
pub struct MemoryResourceBumpStrategy {
    allocated_bytes: usize,
}

impl MemoryResourceStrategy for MemoryResourceBumpStrategy {
    fn allocate(
        &mut self,
        memory: &mut [u8],
        request: MemoryResourceAllocationRequest,
    ) -> Result<MemoryResourceAllocationRegion, MemoryResourceAllocationError> {
        let region = MemoryResourceAllocationRegion::new(
            self.allocated_bytes,
            0,
            alignment_length(self.allocated_bytes, *request.alignment()),
            *request.size(),
            0,
        );

        if usize::from(region.end()) >= memory.len() {
            return Err(MemoryResourceAllocationError::Busy);
        }

        self.allocated_bytes += usize::from(region.total_length());

        Ok(region)
    }

    fn deallocate(
        &mut self,
        _memory: &mut [u8],
        _request: MemoryResourceDeallocationRequest,
    ) -> Result<(), MemoryResourceDeallocationError> {
        Ok(())
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////

#[test]
#[cfg(test)]
fn allocate() {
    use core::num::NonZero;

    let mut memory = [0u8; 64];
    let mut strategy = MemoryResourceBumpStrategy::default();

    let mut expect = |expected, size, alignment| {
        assert_eq!(
            expected,
            NonZero::new(size)
                .zip(NonZero::new(alignment))
                .map(|(size, alignment)| strategy.allocate(
                    &mut memory,
                    MemoryResourceAllocationRequest::new(size, alignment)
                ))
        );
    };

    let mut expect_ok = |start, size, alignment, alignment_length| {
        expect(
            NonZero::new(size).map(|size| {
                Ok(MemoryResourceAllocationRegion::new(
                    start,
                    0,
                    alignment_length,
                    size,
                    0,
                ))
            }),
            size,
            alignment,
        );
    };

    expect_ok(0, 4, 4, 0);
    expect_ok(4, 4, 4, 0);
    expect_ok(8, 4, 4, 0);
    expect_ok(12, 4, 4, 0);
    expect_ok(16, 4, 16, 0);
    expect_ok(20, 4, 16, 12);
    expect_ok(36, 4, 16, 12);
    expect_ok(52, 4, 1, 0);
    expect_ok(56, 4, 2, 0);
    expect_ok(60, 3, 2, 0);
    expect(Some(Err(MemoryResourceAllocationError::Busy)), 1, 1);
}

#[test]
#[cfg(test)]
fn deallocate() {
    use core::num::NonZero;

    let mut memory = [0u8; 64];
    let mut strategy = MemoryResourceBumpStrategy::default();

    assert_eq!(
        Some(true),
        NonZero::new(32)
            .zip(NonZero::new(16))
            .map(|(size, alignment)| strategy
                .allocate(
                    &mut memory,
                    MemoryResourceAllocationRequest::new(size, alignment)
                )
                .is_ok())
    );

    assert_eq!(
        Some(true),
        NonZero::new(1).map(|alignment| {
            strategy
                .deallocate(
                    &mut memory,
                    MemoryResourceDeallocationRequest::new(0, alignment),
                )
                .is_ok()
        }),
    );

    assert_eq!(32, strategy.allocated_bytes(),);
}
