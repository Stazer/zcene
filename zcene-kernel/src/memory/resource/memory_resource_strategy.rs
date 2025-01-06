use crate::memory::resource::{
    MemoryResourceAllocationError, MemoryResourceAllocationRegion, MemoryResourceAllocationRequest,
    MemoryResourceDeallocationError, MemoryResourceDeallocationRequest,
};

////////////////////////////////////////////////////////////////////////////////////////////////////

pub trait MemoryResourceStrategy {
    fn initialize(&mut self, memory: &mut [u8]) {
        memory.fill(0);
    }

    fn allocate(
        &mut self,
        memory: &mut [u8],
        request: MemoryResourceAllocationRequest,
    ) -> Result<MemoryResourceAllocationRegion, MemoryResourceAllocationError>;

    fn deallocate(
        &mut self,
        memory: &mut [u8],
        request: MemoryResourceDeallocationRequest,
    ) -> Result<(), MemoryResourceDeallocationError>;
}
