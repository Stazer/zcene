use alloc::alloc::Global;
use bootloader_api::BootInfo;
use bootloader_api::info::MemoryRegionKind;
use core::alloc::Allocator;
use core::iter::once;
use core::slice::from_raw_parts_mut;
use crate::architecture::FRAME_SIZE;
use crate::architecture::Stack;
use crate::global_allocator::GLOBAL_ALLOCATOR;
use x86_64::registers::control::Cr3;
use x86_64::structures::paging::FrameAllocator;
use x86_64::structures::paging::OffsetPageTable;
use x86_64::structures::paging::Page;
use x86_64::structures::paging::PageTable;
use x86_64::structures::paging::PageTableFlags;
use x86_64::structures::paging::Size4KiB;
use x86_64::structures::paging::{Mapper, PhysFrame};
use x86_64::{PhysAddr, VirtAddr};
use zcene_bare::memory::address::PhysicalMemoryAddress;
use zcene_bare::memory::address::PhysicalMemoryAddressPerspective;
use zcene_bare::memory::address::VirtualMemoryAddress;
use zcene_bare::memory::address::VirtualMemoryAddressPerspective;
use zcene_bare::memory::frame::FrameManager;
use zcene_bare::memory::frame::FrameManagerAllocationError;
use ztd::Method;

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Clone, Debug, Method)]
pub struct KernelMemoryManager {
    physical_memory_offset: u64,
    physical_memory_size_in_bytes: u64,
}

#[derive(Debug)]
pub enum InitializeMemoryManagerError {
    UnsupportedMapping,
    FrameAllocation(FrameManagerAllocationError),
}

impl From<FrameManagerAllocationError> for InitializeMemoryManagerError {
    fn from(error: FrameManagerAllocationError) -> Self {
        Self::FrameAllocation(error)
    }
}

impl KernelMemoryManager {
    pub fn new(boot_info: &mut BootInfo) -> Result<Self, InitializeMemoryManagerError> {
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
            .ok_or(InitializeMemoryManagerError::UnsupportedMapping)?;

        let this = Self {
            physical_memory_offset,
            physical_memory_size_in_bytes,
        };

        for entry in this.active_page_table().iter_mut() {
            let mut flags = entry.flags();

            if flags.contains(PageTableFlags::PRESENT) {
                flags.insert(PageTableFlags::WRITABLE | PageTableFlags::USER_ACCESSIBLE);
                entry.set_flags(flags);
            }
        }

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

        let frame_count = 10 * 1;
        let heap_frame_identifiers = frame_manager.allocate_window(frame_count)?;

        let mut allocator = GLOBAL_ALLOCATOR.inner().lock();

        let memory_address =
            frame_manager.translate_frame_identifier(heap_frame_identifiers.last().unwrap());

        unsafe {
            allocator.init(
                (memory_address.as_u64() + this.physical_memory_offset) as *mut u8,
                frame_count * frame_manager.frame_byte_size(),
            );
        }

        Ok(this)
    }

    pub fn physical_memory(&self) -> &'static mut [u8] {
        unsafe {
            from_raw_parts_mut(
                self.physical_memory_offset as *mut u8,
                self.physical_memory_size_in_bytes.try_into().unwrap(),
            )
        }
    }

    pub fn translate_virtual_memory_address(
        &self,
        memory_address: VirtualMemoryAddress,
    ) -> PhysicalMemoryAddress {
        PhysicalMemoryAddress::from(
            memory_address
                .as_u64()
                .saturating_sub(self.physical_memory_offset),
        )
    }

    pub fn translate_physical_memory_address(
        &self,
        memory_address: PhysicalMemoryAddress,
    ) -> VirtualMemoryAddress {
        VirtualMemoryAddress::from(
            memory_address
                .as_u64()
                .saturating_add(self.physical_memory_offset),
        )
    }

    pub fn frame_manager(&self) -> FrameManager<'static, PhysicalMemoryAddressPerspective> {
        unsafe { FrameManager::new_initialized(FRAME_SIZE, self.physical_memory()) }
    }

    pub fn active_page_table(&self) -> &'static mut PageTable {
        let pointer = self
            .translate_physical_memory_address(PhysicalMemoryAddress::from(
                Cr3::read().0.start_address().as_u64(),
            ))
            .as_u64();

        unsafe { &mut *(pointer as *mut PageTable) }
    }

    pub fn page_table_mapper(&self) -> OffsetPageTable<'static> {
        unsafe {
            OffsetPageTable::new(
                self.active_page_table(),
                VirtAddr::new(self.physical_memory_offset as u64),
            )
        }
    }

    pub fn allocate_stack(&self) -> Option<Stack<VirtualMemoryAddressPerspective>> {
        let stack_frame_count = 4;
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
                        PageTableFlags::PRESENT | PageTableFlags::WRITABLE,
                        &mut EmptyFrameAllocator,
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

pub struct EmptyFrameAllocator;

unsafe impl FrameAllocator<Size4KiB> for EmptyFrameAllocator {
    fn allocate_frame(&mut self) -> Option<PhysFrame> {
        None
    }
}

pub struct FixedFrameAllocator<T>(T)
where
    T: Iterator<Item = PhysFrame>;

unsafe impl<T> FrameAllocator<Size4KiB> for FixedFrameAllocator<T>
where
    T: Iterator<Item = PhysFrame>,
{
    fn allocate_frame(&mut self) -> Option<PhysFrame> {
        self.0.next()
    }
}
