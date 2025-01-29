use crate::architecture::ExecutionUnitIdentifier;
use crate::actor::{ActorThread, ActorThreadType, create_new_stack};
use crate::kernel::Kernel;
use zcene_kernel::memory::address::VirtualMemoryAddress;
use alloc::collections::{BTreeMap, VecDeque};
use ztd::{Method};

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
        stack_pointer: VirtualMemoryAddress
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
            self.queue.push_back(ActorThread::new(
               ActorThreadType::Cooperative,
               None,
            ));
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
            None => None
        };

        match stack_pointer {
            Some(stack_pointer) => {
                stack_pointer
            },
            None => {
                VirtualMemoryAddress::from(create_new_stack(
                    Kernel::get().allocate_stack(),
                ))
            }
        }
    }

    pub fn begin(&mut self, execution_unit_identifier: ExecutionUnitIdentifier) {
        self.threads.insert(execution_unit_identifier, ActorThread::new(
            ActorThreadType::Preemptive ,
            None
        ));
    }

    pub fn end(&mut self, execution_unit_identifier: ExecutionUnitIdentifier) {
        self.threads.insert(execution_unit_identifier, ActorThread::new(
            ActorThreadType::Cooperative,
            None,
        ));
    }
}
