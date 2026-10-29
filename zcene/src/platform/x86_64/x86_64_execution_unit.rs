use crate::common::memory::allocator::ErasedAllocator;
use crate::platform::x86_64::{x86_64_system_call_entry_point, X86_64MemoryManager};
use alloc::boxed::Box;
use x86::current::rflags::RFlags;
use x86::msr::{rdmsr, wrmsr, IA32_EFER, IA32_FMASK, IA32_LSTAR, IA32_STAR};
use x86_64::instructions::segmentation::Segment;
use x86_64::instructions::tables::load_tss;
use x86_64::registers::segmentation::{CS, SS};
use x86_64::structures::gdt::{Descriptor, DescriptorFlags, GlobalDescriptorTable, SegmentSelector};
use x86_64::structures::tss::TaskStateSegment;
use x86_64::VirtAddr;

////////////////////////////////////////////////////////////////////////////////////////////////////

pub struct X86_64ExecutionUnit {
    global_descriptor_table: Box<GlobalDescriptorTable, ErasedAllocator>,

    #[allow(dead_code)]
    task_state_segment: Box<TaskStateSegment, ErasedAllocator>,
    kernel_code_segment_selector: SegmentSelector,
    kernel_data_segment_selector: SegmentSelector,
    task_state_segment_selector: SegmentSelector,
    system_call_target_selectors: u64,
}

unsafe impl Send for X86_64ExecutionUnit {}
unsafe impl Sync for X86_64ExecutionUnit {}

impl X86_64ExecutionUnit {

    pub fn new(memory: &X86_64MemoryManager) -> Option<Self> {
        let privileged_stack = memory.allocate_stack()?.start().as_u64();
        let preemption_stack = memory.allocate_stack()?.start().as_u64();

        let mut global_descriptor_table =
            Box::new_in(GlobalDescriptorTable::new(), memory.heap_allocator());

        let kernel_code_segment_selector = global_descriptor_table
            .append(Descriptor::UserSegment(DescriptorFlags::KERNEL_CODE64.bits()));
        let kernel_data_segment_selector = global_descriptor_table
            .append(Descriptor::UserSegment(DescriptorFlags::KERNEL_DATA.bits()));

        let user_code32_segment_selector = global_descriptor_table
            .append(Descriptor::UserSegment(DescriptorFlags::USER_CODE32.bits()));
        let _user_data_segment_selector = global_descriptor_table
            .append(Descriptor::UserSegment(DescriptorFlags::USER_DATA.bits()));
        let _user_code64_segment_selector = global_descriptor_table
            .append(Descriptor::UserSegment(DescriptorFlags::USER_CODE64.bits()));

        let mut task_state_segment = Box::new_in(TaskStateSegment::new(), memory.heap_allocator());
        task_state_segment.privilege_stack_table[0] = VirtAddr::new(privileged_stack);
        task_state_segment.interrupt_stack_table[0] = VirtAddr::new(preemption_stack);
        task_state_segment.iomap_base = 0xFFFF;

        let task_state_segment_selector = global_descriptor_table
            .append(unsafe { Descriptor::tss_segment_unchecked(&*task_state_segment) });

        let system_call_target_selectors = (u64::from(user_code32_segment_selector.0) << 48)
            | (u64::from(kernel_code_segment_selector.0) << 32);

        Some(Self {
            global_descriptor_table,
            task_state_segment,
            kernel_code_segment_selector,
            kernel_data_segment_selector,
            task_state_segment_selector,
            system_call_target_selectors,
        })
    }

    pub unsafe fn load(&self) {
        unsafe {

            wrmsr(IA32_EFER, rdmsr(IA32_EFER) | 1);

            self.global_descriptor_table.load_unsafe();
            CS::set_reg(self.kernel_code_segment_selector);
            SS::set_reg(self.kernel_data_segment_selector);
            load_tss(self.task_state_segment_selector);

            wrmsr(IA32_STAR, self.system_call_target_selectors);
            wrmsr(IA32_LSTAR, x86_64_system_call_entry_point as *const () as u64);

            wrmsr(IA32_FMASK, RFlags::FLAGS_IF.bits());
        }
    }
}
