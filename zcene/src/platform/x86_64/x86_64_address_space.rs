use crate::platform::unprivileged_address_space::UnprivilegedAddressSpace;

////////////////////////////////////////////////////////////////////////////////////////////////////

pub struct X86_64AddressSpace {
    page_table: u64,
    user_base: u64,
    kernel_base: *mut u8,
    size: usize,
}

unsafe impl Send for X86_64AddressSpace {}
unsafe impl Sync for X86_64AddressSpace {}

impl X86_64AddressSpace {
    pub fn new(page_table: u64, user_base: u64, kernel_base: *mut u8, size: usize) -> Self {
        Self {
            page_table,
            user_base,
            kernel_base,
            size,
        }
    }

    pub fn page_table(&self) -> u64 {
        self.page_table
    }
}

impl UnprivilegedAddressSpace for X86_64AddressSpace {
    fn user_base(&self) -> u64 {
        self.user_base
    }

    fn kernel_base(&self) -> *mut u8 {
        self.kernel_base
    }

    fn size(&self) -> usize {
        self.size
    }
}
