use crate::kernel::memory::KernelMemoryManager;
use crate::memory::address::PhysicalMemoryAddress;
use acpi::{AcpiHandler, PhysicalMapping};
use core::ptr::NonNull;
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
        unsafe {
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
    }

    fn unmap_physical_region<T>(_region: &PhysicalMapping<Self, T>) {}
}
