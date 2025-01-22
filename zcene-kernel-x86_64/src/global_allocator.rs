use linked_list_allocator::LockedHeap;
use core::alloc::GlobalAlloc;
use core::alloc::Layout;
use x86_64::instructions::interrupts::without_interrupts;

////////////////////////////////////////////////////////////////////////////////////////////////////

#[global_allocator]
pub static GLOBAL_ALLOCATOR: GlobalAllocator = GlobalAllocator::empty();

pub struct GlobalAllocator(LockedHeap);

impl GlobalAllocator {
    pub const fn empty() -> Self {
        Self(LockedHeap::empty())
    }

    pub fn inner(&self) -> &LockedHeap {
        &self.0
    }
}

unsafe impl GlobalAlloc for GlobalAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let a = without_interrupts(|| {
            /*use crate::kernel::Kernel;
            use core::fmt::Write;

            Kernel::get()
                .logger()
                .writer(|w| write!(w, "before_alloc\n"));

            let a = self.0.alloc(layout);

            Kernel::get()
                .logger()
                .writer(|w| write!(w, "after_alloc\n"));

            a*/

            self.0.alloc(layout)
        });

        if a.is_null() {
            panic!("No memory");
        }

        a
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        without_interrupts(|| {
            self.0.dealloc(ptr, layout)
        })
    }
}
