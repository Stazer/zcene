use crate::kernel::memory::KernelMemoryManager;
use acpi::{AcpiHandler, PhysicalMapping};
use core::ptr::NonNull;
use zcene_bare::memory::address::PhysicalMemoryAddress;
use ztd::Constructor;

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Constructor, Clone)]
pub struct KernelMemoryManagerAcpiHandler<'a> {
    memory_manager: &'a KernelMemoryManager,
}

impl<'a> AcpiHandler for KernelMemoryManagerAcpiHandler<'a> {
    unsafe fn map_physical_region<T>(
        &self,
        physical_address: usize,
        size: usize,
    ) -> PhysicalMapping<Self, T> {
        let address = self
            .memory_manager
            .translate_physical_memory_address(PhysicalMemoryAddress::from(physical_address));

        PhysicalMapping::new(
            physical_address,
            NonNull::new(address.cast_mut::<T>()).unwrap(),
            size,
            size,
            self.clone(),
        )
    }

    fn unmap_physical_region<T>(region: &PhysicalMapping<Self, T>) {}
}
