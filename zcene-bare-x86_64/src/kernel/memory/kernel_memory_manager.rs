use crate::architecture::Stack;
use crate::architecture::FRAME_SIZE;
use crate::kernel::KERNEL_GLOBAL_ALLOCATOR;
use alloc::alloc::Global;
use bootloader_api::info::MemoryRegionKind;
use bootloader_api::BootInfo;
use core::alloc::Allocator;
use core::iter::once;
use core::slice::from_raw_parts_mut;
use x86_64::registers::control::Cr3;
use x86_64::structures::paging::mapper::MapToError;
use x86_64::structures::paging::page::AddressNotAligned;
use x86_64::structures::paging::page_table::PageTableEntry;
use x86_64::structures::paging::page_table::PageTableLevel;
use x86_64::structures::paging::FrameAllocator;
use x86_64::structures::paging::OffsetPageTable;
use x86_64::structures::paging::Page;
use x86_64::structures::paging::PageSize;
use x86_64::structures::paging::PageTable;
use x86_64::structures::paging::PageTableFlags;
use x86_64::structures::paging::Size4KiB;
use x86_64::structures::paging::{Mapper, PhysFrame};
use x86_64::{PhysAddr, VirtAddr};
use zcene_bare::common::As;
use zcene_bare::memory::address::PhysicalMemoryAddress;
use zcene_bare::memory::address::PhysicalMemoryAddressPerspective;
use zcene_bare::memory::address::VirtualMemoryAddress;
use zcene_bare::memory::address::VirtualMemoryAddressPerspective;
use zcene_bare::memory::frame::FrameManager;
use zcene_bare::memory::frame::FrameManagerAllocationError;
use zcene_bare::memory::region::VirtualMemoryRegion;
use ztd::Method;

////////////////////////////////////////////////////////////////////////////////////////////////////

use zcene_bare::memory::address::{MemoryAddress, MemoryAddressPerspective};

pub const EXECUTION_PAGE_TABLE_INDEX: usize = 256;

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Clone, Debug, Constructor)]
pub struct ActorExecutorStacks {
    kernel_stack: Stack<VirtualMemoryAddressPerspective>,
    actor_stack: Stack<VirtualMemoryAddressPerspective>,
}

#[derive(Clone, Debug, Method)]
pub struct KernelMemoryManager {
    physical_memory_offset: VirtualMemoryAddress,
    physical_memory_size_in_bytes: u64,
    kernel_image_start_virtual_memory_address: VirtualMemoryAddress,
    kernel_image_length: u64,
}

#[derive(Debug)]
pub enum KernelMemoryManagerInitializeError {
    UnsupportedMapping,
    FrameAllocation(FrameManagerAllocationError),
    AddressNotAligned(AddressNotAligned),
    MapToErrorFrameAllocationFailed,
    MapToErrorParentEntryHugePage,
    MapToErrorPageAlreadyMapped,
}

impl From<AddressNotAligned> for KernelMemoryManagerInitializeError {
    fn from(error: AddressNotAligned) -> Self {
        Self::AddressNotAligned(error)
    }
}

impl From<FrameManagerAllocationError> for KernelMemoryManagerInitializeError {
    fn from(error: FrameManagerAllocationError) -> Self {
        Self::FrameAllocation(error)
    }
}

impl<P> From<MapToError<P>> for KernelMemoryManagerInitializeError
where
    P: PageSize,
{
    fn from(error: MapToError<P>) -> Self {
        match error {
            MapToError::FrameAllocationFailed => Self::MapToErrorFrameAllocationFailed,
            MapToError::ParentEntryHugePage => Self::MapToErrorParentEntryHugePage,
            MapToError::PageAlreadyMapped(_) => Self::MapToErrorPageAlreadyMapped,
        }
    }
}

pub struct KernelConfiguration {}

impl KernelMemoryManager {
    pub fn new(
        logger: &crate::kernel::logger::KernelLogger,
        boot_info: &mut BootInfo,
    ) -> Result<Self, KernelMemoryManagerInitializeError> {
        let physical_memory_size_in_bytes = boot_info
            .memory_regions
            .iter()
            .filter(|region| {
                matches!(
                    region.kind,
                    MemoryRegionKind::Usable | MemoryRegionKind::Bootloader
                )
            })
            .map(|region| region.end.saturating_sub(region.start))
            .sum::<u64>();

        let physical_memory_offset = boot_info
            .physical_memory_offset
            .into_option()
            .ok_or(KernelMemoryManagerInitializeError::UnsupportedMapping)?;

        let this = Self {
            physical_memory_offset: VirtualMemoryAddress::from(physical_memory_offset),
            physical_memory_size_in_bytes,
            kernel_image_length: boot_info.kernel_len,
            kernel_image_start_virtual_memory_address: VirtualMemoryAddress::from(
                physical_memory_offset + boot_info.kernel_addr,
            ),
        };

        let mut frame_manager = this.frame_manager();

        for memory_region in boot_info.memory_regions.iter() {
            if matches!(memory_region.kind, MemoryRegionKind::Usable) {
                continue;
            }

            let start = frame_manager
                .translate_memory_address(PhysicalMemoryAddress::from(memory_region.start));
            let end = frame_manager
                .translate_memory_address(PhysicalMemoryAddress::from(memory_region.end));

            for identifier in start..end {
                if frame_manager
                    .translate_frame_identifier(identifier)
                    .as_u64()
                    >= physical_memory_size_in_bytes
                {
                    continue;
                }

                match frame_manager.allocate_frames(once(identifier)) {
                    Ok(()) => {}
                    Err(FrameManagerAllocationError::Allocated(_)) => {}
                    Err(error) => todo!(),
                }
            }
        }

        /*let p1 = |address: u64| {
            (address >> 12) as u16
        };

        let p2 = |address: u64| {
            (address >> 12 >> 9)as u16
        };

        let p3 = |address: u64| {
            (address >> 12 >> 9 >> 9)as u16
        };

        let p4 = |address: u64| {
            (address >> 12 >> 9 >> 9 >> 9)as u16
        };

        let forge_address = |p4: u16, p3: u16, p2: u16, p1: u16, index: u16| -> u64 {
            (p4 as u64) << (9 + 9 + 9 + 12) |
            (p3 as u64) << (9 + 9 + 12) |
            (p2 as u64) << (9 + 12) |
            (p1 as u64) << 12 |
            (index as u64)
        };

        use core::fmt::Write;

        let b = Cr3::read_raw();

        let page_table = this.active_page_table();

        let mut frame_manager = this.frame_manager();
        let a = frame_manager.unallocated_frame_identifiers().next().unwrap();
        let execution_page_table = frame_manager.allocate_frames(once(a)).unwrap();

        // Execution Stack
        page_table[256].set_addr(PhysAddr::new(frame_manager.translate_frame_identifier(a).as_u64()), PageTableFlags::PRESENT);
        //page_table[256].set_addr(b.0.start_address(), PageTableFlags::PRESENT | PageTableFlags::HUGE_PAGE | PageTableFlags::NO_EXECUTE);
        //page_table[257].set_addr(b.0.start_address(), PageTableFlags::PRESENT | PageTableFlags::WRITABLE | PageTableFlags::HUGE_PAGE | PageTableFlags::USER_ACCESSIBLE | PageTableFlags::NO_EXECUTE);
        //page_table[258].set_addr(b.0.start_address(), PageTableFlags::PRESENT | PageTableFlags::HUGE_PAGE | PageTableFlags::NO_EXECUTE);

        // User Code (variable)
        //page_table[259].set_addr(b.0.start_address(), PageTableFlags::PRESENT | PageTableFlags::HUGE_PAGE | PageTableFlags::USER_ACCESSIBLE);

        let a = Cr3::read_raw();
        unsafe {
            Cr3::write_raw(a.0, a.1);
        }

        for (i, entry) in page_table.iter().enumerate() {
            if !entry.flags().contains(PageTableFlags::PRESENT) {
                continue;
            }

            logger.writer(|w| write!(w, "[{}] {:?}\n", i, entry));
        }

        logger.writer(|w| write!(w, "\n"));

        for a in [256, 257, 258].iter() {
            let user_stack_from = VirtAddr::new_truncate(
                forge_address(
                    511,
                    511,
                    *a,
                    0,
                    0,
                )
            );

            let user_stack_to = VirtAddr::new_truncate(
                forge_address(
                    511,
                    511,
                    *a,
                    (2i32.pow(9) - 1) as u16,
                    (2i32.pow(12) - 1) as u16,
                )
            );

            logger.writer(|w| write!(w, "\n{:?} -> {:?}\n", user_stack_from, user_stack_to));
        }

        loop {}

        let address = VirtAddr::new_truncate(
            forge_address(
                511,
                0,
                511,
                511,
                0,
            )
        );

        logger.writer(|w| write!(w, "\n{:?}\n", address));

        let mut current_table = this.active_page_table();

        for (i, index) in [address.p4_index(), address.p3_index(), address.p2_index(), address.p1_index()].iter().enumerate() {
            logger.writer(|w| write!(w, "{:?}; ", index));
        }

        logger.writer(|w| write!(w, "\n"));

        for (i, index) in [address.p4_index(), address.p3_index(), address.p2_index(), address.p1_index()].iter().enumerate() {
            let entry = current_table[*index].clone();

            logger.writer(|w| write!(w, "[{:?}] {:?}\n", index, entry));

            if !entry.flags().contains(PageTableFlags::PRESENT) {
                break;
            }
        }

        loop {}*/

        // Physical Region

        // Kernel Code
        // Kernel Heap

        // CPU-centric Kernel Stacks + Guards

        // Actor-centric Heap
        // Actor-centric Code
        // Actor-centric Stack + Guards
        //
        // Actor-centric Stack Guard
        // Actor-centric Stack Data (n * 4096)
        // Actor-centric Stack Guard
        // Actor-centric Kernel System Call Stack
        // Actor-centric Stack Guard

        let mut mapper = this.page_table_mapper();

        let start_address = 0x0000_0000_FFFF_0000u64;
        let frame_count = 10000;

        let mut current_address = start_address;

        let page_table = this.active_page_table();

        for entry in page_table.iter_mut() {
            if !entry.flags().contains(PageTableFlags::PRESENT) {
                continue;
            }

            entry.set_flags(entry.flags() | PageTableFlags::USER_ACCESSIBLE);

            let l2 = unsafe {
                this.translate_physical_memory_address(PhysicalMemoryAddress::from(
                    entry.addr().as_u64(),
                ))
                .cast_mut::<PageTable>()
                .as_mut()
                .unwrap()
            };

            for entry2 in l2.iter_mut() {
                if !entry2.flags().contains(PageTableFlags::PRESENT)
                    || entry2.flags().contains(PageTableFlags::HUGE_PAGE)
                {
                    continue;
                }

                entry2.set_flags(entry2.flags() | PageTableFlags::USER_ACCESSIBLE);

                let l3 = unsafe {
                    this.translate_physical_memory_address(PhysicalMemoryAddress::from(
                        entry2.addr().as_u64(),
                    ))
                    .cast_mut::<PageTable>()
                    .as_mut()
                    .unwrap()
                };

                for entry3 in l3.iter_mut() {
                    if !entry3.flags().contains(PageTableFlags::PRESENT)
                        || entry3.flags().contains(PageTableFlags::HUGE_PAGE)
                    {
                        continue;
                    }

                    entry3.set_flags(entry3.flags() | PageTableFlags::USER_ACCESSIBLE);

                    let l4 = unsafe {
                        this.translate_physical_memory_address(PhysicalMemoryAddress::from(
                            entry3.addr().as_u64(),
                        ))
                        .cast_mut::<PageTable>()
                        .as_mut()
                        .unwrap()
                    };

                    for entry4 in l4.iter_mut() {
                        if !entry4.flags().contains(PageTableFlags::PRESENT) {
                            continue;
                        }

                        entry4.set_flags(entry4.flags() | PageTableFlags::USER_ACCESSIBLE);
                    }
                }
            }
        }

        let a = Cr3::read_raw();
        unsafe {
            Cr3::write_raw(a.0, a.1);
        }

        for (i, heap_frame_identifier) in frame_manager
            .unallocated_frame_identifiers()
            .take(frame_count)
            .enumerate()
        {
            let physical_address = frame_manager.translate_frame_identifier(heap_frame_identifier);

            this.frame_manager()
                .allocate_frames(once(heap_frame_identifier))
                .unwrap();

            unsafe {
                mapper
                    .map_to(
                        Page::<Size4KiB>::from_start_address_unchecked(VirtAddr::new(
                            current_address,
                        )),
                        PhysFrame::from_start_address(PhysAddr::new(physical_address.as_u64()))?,
                        PageTableFlags::PRESENT | PageTableFlags::WRITABLE,
                        &mut KernelMemoryManagerFrameAllocator::new(&this),
                    )?
                    .flush();
            }

            current_address += frame_manager.frame_byte_size().r#as();
        }

        let mut allocator = KERNEL_GLOBAL_ALLOCATOR.inner().lock();

        unsafe {
            allocator.init(
                VirtualMemoryAddress::from(start_address).cast_mut(),
                frame_count * frame_manager.frame_byte_size(),
            );
        }

        Ok(this)
    }

    fn physical_memory(&self) -> &'static mut [u8] {
        unsafe {
            from_raw_parts_mut(
                self.physical_memory_offset.cast_mut::<u8>(),
                self.physical_memory_size_in_bytes.r#as(),
            )
        }
    }

    pub fn kernel_image_virtual_memory_region(&self) -> VirtualMemoryRegion {
        VirtualMemoryRegion::new(
            self.kernel_image_start_virtual_memory_address,
            self.kernel_image_length.r#as(),
        )
    }

    pub fn translate_virtual_memory_address(
        &self,
        memory_address: VirtualMemoryAddress,
    ) -> PhysicalMemoryAddress {
        PhysicalMemoryAddress::from(
            memory_address
                .as_u64()
                .saturating_sub(self.physical_memory_offset.as_u64()),
        )
    }

    pub fn translate_physical_memory_address(
        &self,
        memory_address: PhysicalMemoryAddress,
    ) -> VirtualMemoryAddress {
        VirtualMemoryAddress::from(
            memory_address
                .as_u64()
                .saturating_add(self.physical_memory_offset.as_u64()),
        )
    }

    pub fn frame_manager(&self) -> FrameManager<'static, PhysicalMemoryAddressPerspective> {
        unsafe { FrameManager::new_initialized(FRAME_SIZE, self.physical_memory()) }
    }

    fn active_page_table(&self) -> &'static mut PageTable {
        let pointer = self
            .translate_physical_memory_address(PhysicalMemoryAddress::from(
                Cr3::read().0.start_address().as_u64(),
            ))
            .as_u64();

        unsafe { &mut *(pointer as *mut PageTable) }
    }

    fn active_root_page_table(&self) -> &'static mut PageTable {
        let pointer = self
            .translate_physical_memory_address(PhysicalMemoryAddress::from(
                Cr3::read().0.start_address().as_u64(),
            ))
            .as_u64();

        unsafe { &mut *(pointer as *mut PageTable) }
    }

    fn active_execution_page_table(&self) -> &'static mut PageTable {
        unsafe {
            self.translate_physical_memory_address(PhysicalMemoryAddress::from(
                self.active_root_page_table()[EXECUTION_PAGE_TABLE_INDEX]
                    .addr()
                    .as_u64(),
            ))
            .cast_mut::<PageTable>()
            .as_mut()
            .unwrap()
        }
    }

    fn page_table_mapper(&self) -> OffsetPageTable<'static> {
        unsafe {
            OffsetPageTable::new(
                self.active_page_table(),
                VirtAddr::new(self.physical_memory_offset.as_usize().r#as()),
            )
        }
    }

    pub fn allocate_user_heap(&self) -> Option<VirtualMemoryAddress> {
        None
    }

    pub fn allocate_actor_executor_stacks(&self) -> Option<ActorExecutorStacks> {
        /*for (i, heap_frame_identifier) in frame_manager
            .unallocated_frame_identifiers()
            .take(frame_count)
            .enumerate()
        {
            let physical_address = frame_manager.translate_frame_identifier(heap_frame_identifier);

            this.frame_manager()
                .allocate_frames(once(heap_frame_identifier))
                .unwrap();

            unsafe {
                mapper
                    .map_to(
                        Page::<Size4KiB>::from_start_address_unchecked(VirtAddr::new(
                            current_address,
                        )),
                        PhysFrame::from_start_address(PhysAddr::new(physical_address.as_u64()))?,
                        PageTableFlags::PRESENT | PageTableFlags::WRITABLE,
                        &mut KernelMemoryManagerFrameAllocator::new(&this),
                    )?
                    .flush();
            }

            current_address += frame_manager.frame_byte_size().r#as();
        }*/

        /*let stack_frame_count = 4;
        let stack_size = stack_frame_count * FRAME_SIZE;

        let mut first_address = 0;
        let mut stack_address = 0;
        let mut mapper = self.page_table_mapper();

        for stack_frame_identifier in self.frame_manager().allocate_window(4).unwrap() {
            stack_address = self
                .frame_manager()
                .translate_frame_identifier(stack_frame_identifier)
                .as_usize();

            let page = Page::<Size4KiB>::containing_address(VirtAddr::new(
                (stack_address).try_into().unwrap(),
            ));

            if first_address == 0 {
                first_address = page.start_address().as_u64() + stack_size as u64;
            }

            unsafe {
                mapper
                    .map_to(
                        page,
                        PhysFrame::from_start_address(PhysAddr::new(
                            stack_address.try_into().unwrap(),
                        ))
                        .unwrap(),
                        PageTableFlags::PRESENT
                            | PageTableFlags::WRITABLE
                            | PageTableFlags::USER_ACCESSIBLE
                            | PageTableFlags::NO_EXECUTE,
                        &mut KernelMemoryManagerFrameAllocator::new(self),
                    )
                    .expect("Hello World")
                    .flush();
            }
        }

        let first_address = VirtualMemoryAddress::from(first_address);*/

        //Some(Stack::new(first_address, stack_size))


        todo!()
    }

    pub fn allocate_user_stack(&self) -> Option<Stack<VirtualMemoryAddressPerspective>> {
        let stack_frame_count = 8;
        let stack_size = stack_frame_count * FRAME_SIZE;

        let mut first_address = 0;
        let mut stack_address = 0;
        let mut mapper = self.page_table_mapper();

        for stack_frame_identifier in self.frame_manager().allocate_window(8).unwrap() {
            stack_address = self
                .frame_manager()
                .translate_frame_identifier(stack_frame_identifier)
                .as_usize();

            let page = Page::<Size4KiB>::containing_address(VirtAddr::new(
                (stack_address).try_into().unwrap(),
            ));

            if first_address == 0 {
                first_address = page.start_address().as_u64() + stack_size as u64;
            }

            unsafe {
                mapper
                    .map_to(
                        page,
                        PhysFrame::from_start_address(PhysAddr::new(
                            stack_address.try_into().unwrap(),
                        ))
                        .unwrap(),
                        PageTableFlags::PRESENT
                            | PageTableFlags::WRITABLE
                            | PageTableFlags::USER_ACCESSIBLE
                            | PageTableFlags::NO_EXECUTE,
                        &mut KernelMemoryManagerFrameAllocator::new(self),
                    )
                    .expect("Hello World")
                    .flush();
            }
        }

        let first_address = VirtualMemoryAddress::from(first_address);

        Some(Stack::new(first_address, stack_size))
    }

    pub fn allocate_stack(&self) -> Option<Stack<VirtualMemoryAddressPerspective>> {
        let stack_frame_count = 8;
        let stack_size = stack_frame_count * FRAME_SIZE;

        let mut first_address = 0;
        let mut stack_address = 0;
        let mut mapper = self.page_table_mapper();

        for stack_frame_identifier in self.frame_manager().allocate_window(8).unwrap() {
            stack_address = self
                .frame_manager()
                .translate_frame_identifier(stack_frame_identifier)
                .as_usize();

            let page = Page::<Size4KiB>::containing_address(VirtAddr::new(
                (stack_address).try_into().unwrap(),
            ));

            if first_address == 0 {
                first_address = page.start_address().as_u64() + stack_size as u64;
            }

            unsafe {
                mapper
                    .map_to(
                        page,
                        PhysFrame::from_start_address(PhysAddr::new(
                            stack_address.try_into().unwrap(),
                        ))
                        .unwrap(),
                        PageTableFlags::PRESENT | PageTableFlags::WRITABLE,
                        &mut KernelMemoryManagerFrameAllocator::new(self),
                    )
                    .expect("Hello World")
                    .flush();
            }
        }

        let first_address = VirtualMemoryAddress::from(first_address);

        Some(Stack::new(first_address, stack_size))
    }

    pub fn heap_allocator(&self) -> impl Allocator {
        Global
    }
}

use ztd::Constructor;

#[derive(Constructor)]
pub struct KernelMemoryManagerFrameAllocator<'a> {
    memory_manager: &'a KernelMemoryManager,
}

unsafe impl FrameAllocator<Size4KiB> for KernelMemoryManagerFrameAllocator<'_> {
    fn allocate_frame(&mut self) -> Option<PhysFrame> {
        let frame_identifier = self
            .memory_manager
            .frame_manager()
            .unallocated_frame_identifiers()
            .next()?;
        self.memory_manager
            .frame_manager()
            .allocate_frames(once(frame_identifier))
            .ok()?;

        Some(
            PhysFrame::from_start_address(PhysAddr::new(
                self.memory_manager
                    .frame_manager()
                    .translate_frame_identifier(frame_identifier)
                    .as_u64(),
            ))
            .ok()?,
        )
    }
}

use x86_64::structures::paging::FrameDeallocator;

#[derive(Constructor)]
pub struct KernelMemoryManagerFrameDeallocator<'a> {
    memory_manager: &'a KernelMemoryManager,
}

impl FrameDeallocator<Size4KiB> for KernelMemoryManagerFrameDeallocator<'_> {
    unsafe fn deallocate_frame(&mut self, frame: PhysFrame<Size4KiB>) {
        let frame_identifier = self
            .memory_manager
            .frame_manager()
            .translate_memory_address(PhysicalMemoryAddress::from(frame.start_address().as_u64()));

        self.memory_manager
            .frame_manager()
            .deallocate_frames(once(frame_identifier))
            .unwrap();
    }
}
