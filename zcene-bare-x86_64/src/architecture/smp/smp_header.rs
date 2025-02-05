use x86_64::structures::DescriptorTablePointer;
use x86_64::VirtAddr;
use ztd::Method;

////////////////////////////////////////////////////////////////////////////////////////////////////

#[repr(C, packed)]
#[derive(Method)]
pub struct SMPHeader {
    pub stack_start: u64,
    pub stack_size: u64,
    pub page_table_start: u32,
    pub gdt64_pointer: DescriptorTablePointer,
}

////////////////////////////////////////////////////////////////////////////////////////////////////

#[no_mangle]
#[link_section = ".smp_header_section"]
pub static mut SMP_HEADER: SMPHeader = SMPHeader {
    stack_start: 0,
    stack_size: 0,
    page_table_start: 0,
    gdt64_pointer: DescriptorTablePointer {
        limit: 0,
        base: VirtAddr::zero(),
    },
};
