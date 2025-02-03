use crate::actor::{ActorThread, ActorThreadType};
use crate::architecture::ExecutionUnitIdentifier;
use crate::kernel::Kernel;
use alloc::collections::{BTreeMap, VecDeque};
use x86_64::registers::rflags::RFlags;
use x86_64::structures::gdt::SegmentSelector;
use x86_64::PrivilegeLevel;
use zcene_kernel::memory::address::VirtualMemoryAddress;
use ztd::Method;

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Default, Method)]
#[Method(all)]
pub struct ActorThreadScheduler {
    queue: VecDeque<ActorThread>,
    threads: BTreeMap<ExecutionUnitIdentifier, ActorThread>,
}

use core::fmt::Write;

impl ActorThreadScheduler {
    pub fn r#break(
        &mut self,
        execution_unit_identifier: ExecutionUnitIdentifier,
        stack_pointer: VirtualMemoryAddress,
    ) {
        let mut thread = match self.threads.remove(&execution_unit_identifier) {
            Some(thread) => thread,
            None => return,
        };

        thread.set_stack_pointer(Some(stack_pointer));
        self.queue.push_back(thread);

        if self
            .queue
            .iter()
            .filter(|x| matches!(x.r#type(), ActorThreadType::Cooperative))
            .count()
            < 1
        {
            self.queue
                .push_back(ActorThread::new(ActorThreadType::Cooperative, None));
        }
    }

    pub fn r#continue(
        &mut self,
        execution_unit_identifier: ExecutionUnitIdentifier,
        next_thread: Option<ActorThread>,
    ) -> VirtualMemoryAddress {
        let stack_pointer = match next_thread {
            Some(thread) => {
                let stack_pointer = *thread.stack_pointer();
                self.threads.insert(execution_unit_identifier, thread);

                stack_pointer
            }
            None => None,
        };

        match stack_pointer {
            Some(stack_pointer) => stack_pointer,
            None => {
                //let mut stack = create_new_stack(Kernel::get().memory_manager().allocate_stack().unwrap().current_memory_address().as_u64());

                let mut stack = Kernel::get().memory_manager().allocate_stack().unwrap(); //.current_memory_address().as_u64();
                                                                                          //crate::common::println!("from... {:?}", stack);

                stack.push_interrupt_frame(
                    RFlags::empty(),
                    crate::actor::hello,
                    SegmentSelector::new(1, PrivilegeLevel::Ring0),
                    SegmentSelector::new(2, PrivilegeLevel::Ring0),
                );
                //crate::common::println!("to... {:?}", stack);

                crate::common::println!("create new stack...");

                *stack.current_memory_address()
                //*stack.current_memory_address()
            }
        }
    }

    pub fn begin(&mut self, execution_unit_identifier: ExecutionUnitIdentifier) {
        self.threads.insert(
            execution_unit_identifier,
            ActorThread::new(ActorThreadType::Preemptive, None),
        );
    }

    pub fn end(&mut self, execution_unit_identifier: ExecutionUnitIdentifier) {
        self.threads.insert(
            execution_unit_identifier,
            ActorThread::new(ActorThreadType::Cooperative, None),
        );
    }
}

#[inline(never)]
extern "C" fn create_new_stack(mut new_stack_pointer: u64) -> u64 {
    unsafe {
        core::arch::asm!(
            "mov rbx, rsp",

            "mov rsp, {new_stack_pointer}",

            "push {stack_segment:r}",
            "push {new_stack_pointer}",
            "push {rflags}",
            "push {code_segment:r}",
            "push {new_instruction_pointer}",

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

            "mov {new_stack_pointer}, rsp",

            "mov rsp, rbx",

            rflags = in(reg) RFlags::empty().bits(),
            new_instruction_pointer = in(reg) x86_64::VirtAddr::new(crate::actor::hello as _).as_u64(),
            new_stack_pointer = inout(reg) new_stack_pointer,
            code_segment = in(reg) SegmentSelector::new(1, PrivilegeLevel::Ring0).0,
            stack_segment = in(reg) SegmentSelector::new(2, PrivilegeLevel::Ring0).0,
        )
    }

    new_stack_pointer
}
