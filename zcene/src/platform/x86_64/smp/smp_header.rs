use x86_64::VirtAddr;
use x86_64::structures::DescriptorTablePointer;
use ztd::Method;
use core::mem::offset_of;

////////////////////////////////////////////////////////////////////////////////////////////////////

#[repr(C, packed)]
#[derive(Method)]
pub struct SMPHeader {
    pub stack_start: u64,
    pub stack_size: u64,
    pub page_table_start: u32,
    pub gdt64_pointer: DescriptorTablePointer,
    pub cr4: u64,
}

////////////////////////////////////////////////////////////////////////////////////////////////////

const _: () = assert!(offset_of!(SMPHeader, stack_size) == 0x08);
const _: () = assert!(offset_of!(SMPHeader, page_table_start) == 0x10);
const _: () = assert!(offset_of!(SMPHeader, gdt64_pointer) == 0x14);
const _: () = assert!(offset_of!(SMPHeader, cr4) == 0x1e);

////////////////////////////////////////////////////////////////////////////////////////////////////

#[unsafe(no_mangle)]
#[unsafe(link_section = ".smp_header_section")]
pub static mut SMP_HEADER: SMPHeader = SMPHeader {
    stack_start: 0,
    stack_size: 0,
    page_table_start: 0,
    gdt64_pointer: DescriptorTablePointer {
        limit: 0,
        base: VirtAddr::zero(),
    },
    cr4: 0,
};
