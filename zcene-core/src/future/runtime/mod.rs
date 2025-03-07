mod future_runtime;
mod future_runtime_actor_environment;
mod future_runtime_allocator;
mod future_runtime_box_future;
mod future_runtime_common_bounds;
mod future_runtime_concurrent_queue;
mod future_runtime_continue_waker;
mod future_runtime_create_error;
mod future_runtime_future;
mod future_runtime_handler;
mod future_runtime_no_operation_yielder;
mod future_runtime_queue;
mod future_runtime_reference;
mod future_runtime_spawn_error;
mod future_runtime_task;
mod future_runtime_task_reference;
mod future_runtime_waker;
mod future_runtime_yielder;

pub use future_runtime::*;
pub use future_runtime_actor_environment::*;
pub use future_runtime_allocator::*;
pub use future_runtime_box_future::*;
pub use future_runtime_common_bounds::*;
pub use future_runtime_concurrent_queue::*;
pub use future_runtime_continue_waker::*;
pub use future_runtime_create_error::*;
pub use future_runtime_future::*;
pub use future_runtime_handler::*;
pub use future_runtime_no_operation_yielder::*;
pub use future_runtime_queue::*;
pub use future_runtime_reference::*;
pub use future_runtime_spawn_error::*;
pub use future_runtime_task::*;
pub use future_runtime_task_reference::*;
pub use future_runtime_waker::*;
pub use future_runtime_yielder::*;

////////////////////////////////////////////////////////////////////////////////////////////////////

use ztd::Constructor;

#[derive(Constructor)]
pub struct FutureRuntimeInlineHandler<A, Q, Y, W>
where
    A: FutureRuntimeAllocator,
    Q: FutureRuntimeQueue<Self>,
    Y: FutureRuntimeYielder,
    W: FutureRuntimeWaker<Self>,
{
    allocator: A,
    queue: Q,
    yielder: Y,
    waker: W,
}

impl<A, Q, Y, W> FutureRuntimeHandler for FutureRuntimeInlineHandler<A, Q, Y, W>
where
    A: FutureRuntimeAllocator,
    Q: FutureRuntimeQueue<Self>,
    Y: FutureRuntimeYielder,
    W: FutureRuntimeWaker<Self>,
{
    type Allocator = A;
    type Queue = Q;
    type Yielder = Y;
    type Waker = W;
    type Specification = ();
    type Data = ();

    fn allocator(&self) -> &Self::Allocator {
        &self.allocator
    }

    fn queue(&self) -> &Self::Queue {
        &self.queue
    }

    fn yielder(&self) -> &Self::Yielder {
        &self.yielder
    }

    fn waker(&self) -> &Self::Waker {
        &self.waker
    }
}
