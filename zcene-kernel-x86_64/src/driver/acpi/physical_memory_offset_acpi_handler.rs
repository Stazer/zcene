use acpi::hpet::HpetTable;
use acpi::{AcpiHandler, AcpiTables, HpetInfo, PhysicalMapping};
use core::mem::size_of;
use core::ptr::NonNull;
use ztd::{Constructor, Method};

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Constructor, Clone, Debug, Method)]
pub struct PhysicalMemoryOffsetAcpiHandler {
    physical_memory_offset: usize,
}

impl AcpiHandler for PhysicalMemoryOffsetAcpiHandler {
    unsafe fn map_physical_region<T>(
        &self,
        physical_address: usize,
        size: usize,
    ) -> PhysicalMapping<Self, T> {
        PhysicalMapping::new(
            physical_address,
            NonNull::new((physical_address + self.physical_memory_offset) as *mut T).unwrap(),
            size,
            size,
            self.clone(),
        )
    }

    fn unmap_physical_region<T>(region: &PhysicalMapping<Self, T>) {}
}
