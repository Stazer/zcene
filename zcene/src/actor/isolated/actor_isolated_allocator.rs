use core::alloc::{GlobalAlloc, Layout};
use core::ptr::addr_of_mut;
use linked_list_allocator::LockedHeap;

////////////////////////////////////////////////////////////////////////////////////////////////////

pub const ACTOR_ISOLATED_HEAP_SIZE: usize = 64 * 1024;

static mut HEAP: [u8; ACTOR_ISOLATED_HEAP_SIZE] = [0; ACTOR_ISOLATED_HEAP_SIZE];

/// The global allocator of an isolated program: a heap in the program's own `.bss`.
pub struct ActorIsolatedAllocator {
    heap: LockedHeap,
}

impl ActorIsolatedAllocator {
    pub const fn new() -> Self {
        Self {
            heap: LockedHeap::empty(),
        }
    }

    pub(crate) fn initialize(&self) {
        // SAFETY: called once, before the actor allocates anything, and the heap is not used
        // for anything else.
        unsafe {
            self.heap
                .lock()
                .init(addr_of_mut!(HEAP).cast::<u8>(), ACTOR_ISOLATED_HEAP_SIZE);
        }
    }
}

unsafe impl GlobalAlloc for ActorIsolatedAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        unsafe { self.heap.alloc(layout) }
    }

    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        unsafe { self.heap.dealloc(pointer, layout) }
    }
}
