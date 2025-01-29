use zcene_kernel::memory::address::{
    MemoryAddress,
    MemoryAddressPerspective,
    VirtualMemoryAddressPerspective,
    VirtualMemoryAddress,
};
use ztd::Method;
use x86_64::registers::rflags::RFlags;
use x86_64::structures::gdt::SegmentSelector;
use x86_64::PrivilegeLevel;
use core::arch::asm;
use x86_64::VirtAddr;

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Method)]
#[Method(accessors)]
pub struct Stack<P>
where
    P: MemoryAddressPerspective
{
    initial_memory_address: MemoryAddress<P>,
    current_memory_address: MemoryAddress<P>,
    total_size: usize,
}

impl<P> Stack<P>
where
    P: MemoryAddressPerspective
{
    pub fn new(
        memory_address: MemoryAddress<P>,
        total_size: usize,
    ) -> Self {
        Self {
            initial_memory_address: memory_address,
            current_memory_address: memory_address,
            total_size,
        }
    }
}

impl Stack<VirtualMemoryAddressPerspective> {
    pub fn push_interrupt_frame(
        &mut self,
        rflags: RFlags,
        function: fn() -> !,
        //stack_memory_address: VirtualMemoryAddress,
        code_segment: SegmentSelector,
        stack_segment: SegmentSelector,
    ) {
        let mut current_memory_address = self.current_memory_address.as_u64();

        unsafe {
            asm!(
                "mov rbx, rsp",

                "mov rsp, {current_stack_pointer}",

                "push {stack_segment:r}",
                "push {current_stack_pointer}",
                "push {rflags}",
                "push {code_segment:r}",
                "push {instruction_pointer}",

                "push 0",
                "push 0",
                "push 0",
                "push 0",
                "push 0",
                "push 0",
                "push 0",
                "push 0",
                "push 0",
                "push 0",
                "push 0",
                "push 0",
                "push 0",
                "push 0",
                "push 0",

                "mov {current_stack_pointer}, rsp",

                "mov rsp, rbx",

                rflags = in(reg) rflags.bits(),
                instruction_pointer = in(reg) VirtualMemoryAddress::from(function as u64).as_u64(),
                current_stack_pointer = inout(reg) current_memory_address,
                code_segment = in(reg) code_segment.0,
                stack_segment = in(reg) stack_segment.0,
            )
        }

        self.current_memory_address = VirtualMemoryAddress::from(current_memory_address);
    }
}
