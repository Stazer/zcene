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
                let mut stack = Kernel::get().memory_manager().allocate_stack().unwrap();

                stack.push_interrupt_frame(
                    RFlags::empty(),
                    crate::actor::hello,
                    SegmentSelector::new(1, PrivilegeLevel::Ring0),
                    SegmentSelector::new(2, PrivilegeLevel::Ring0),
                );

                *stack.current_memory_address()
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
