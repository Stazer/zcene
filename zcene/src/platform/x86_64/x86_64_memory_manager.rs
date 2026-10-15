use crate::platform::x86_64::X86_64MemoryManagerInitializeError;
use crate::platform::x86_64::FRAME_SIZE;
use crate::common::As;
use crate::common::memory::address::DefaultMemoryAddressTransformer;
use crate::common::memory::address::PhysicalMemoryAddress;
use crate::common::memory::address::PhysicalMemoryAddressPerspective;
use crate::common::memory::address::VirtualMemoryAddress;
use crate::common::memory::allocator::LinkedListHeapMemoryAllocator;
use crate::common::memory::allocator::SelfContainedHeapAllocator;
use crate::common::memory::FrameIdentifierMemoryAddressTranslator;
use crate::common::memory::allocator::FrameManager;
use crate::common::memory::region::VirtualMemoryRegion;
use bootloader_api::BootInfo;
use bootloader_api::info::MemoryRegionKind;
use core::fmt::Write;
use core::iter::once;
use core::slice::from_raw_parts_mut;
use linked_list_allocator::LockedHeap;
use x86_64::registers::control::Cr3;
use x86_64::structures::paging::FrameAllocator;
use x86_64::structures::paging::OffsetPageTable;
use x86_64::structures::paging::Page;
use x86_64::structures::paging::PageTable;
use x86_64::structures::paging::PageTableFlags;
use x86_64::structures::paging::Size4KiB;
use x86_64::structures::paging::{Mapper, PhysFrame};
use x86_64::{PhysAddr, VirtAddr};
use ztd::Method;

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Constructor, Debug)]
pub struct X86_64MemoryManagerAddressTranslator {
    physical_memory_offset: VirtualMemoryAddress,
}

impl X86_64MemoryManagerAddressTranslator {
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
}

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Method)]
pub struct X86_64MemoryManager {
    physical_memory_offset: VirtualMemoryAddress,
    physical_memory_size_in_bytes: u64,
    managed_memory_size: u64,
    usable_memory_in_bytes: u64,
    kernel_image_start_virtual_memory_address: VirtualMemoryAddress,
    kernel_image_length: u64,
    frame_manager: FrameManager<'static, PhysicalMemoryAddressPerspective>,
    heap_allocator_reference: Option<ErasedAllocator>,
}

impl X86_64MemoryManager {
    pub fn new(
        boot_info: &mut BootInfo,
        logger: &crate::common::logger::LoggerService,
    ) -> Result<Self, X86_64MemoryManagerInitializeError> {
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

        let highest_physical_address = boot_info
            .memory_regions
            .iter()
            .map(|region| region.end)
            .max()
            .unwrap_or(0);

        let highest_ram_address = boot_info
            .memory_regions
            .iter()
            .filter(|region| matches!(region.kind, MemoryRegionKind::Usable | MemoryRegionKind::Bootloader))
            .map(|region| region.end)
            .max()
            .unwrap_or(0);

        let highest_usable_address = boot_info
            .memory_regions
            .iter()
            .filter(|region| matches!(region.kind, MemoryRegionKind::Usable))
            .map(|region| region.end)
            .max()
            .unwrap_or(0);

        let bitmap_size_in_bytes = (highest_usable_address as usize).div_ceil(FRAME_SIZE * 8);
        let bitmap_frames = bitmap_size_in_bytes.div_ceil(FRAME_SIZE);

        let mut w = logger.writer();
        let _ = write!(w, "Highest physical address: {:#x}\n", highest_physical_address);
        let _ = write!(w, "Highest RAM address: {:#x}\n", highest_ram_address);
        let _ = write!(w, "Highest usable address: {:#x}\n", highest_usable_address);
        let _ = write!(w, "Bitmap: {} bytes, {} frames\n", bitmap_size_in_bytes, bitmap_frames);

        let physical_memory_offset = VirtualMemoryAddress::from(
            boot_info
                .physical_memory_offset
                .into_option()
                .ok_or(X86_64MemoryManagerInitializeError::UnsupportedMapping)?
        );

        let frame_identifier_address_translator = FrameIdentifierMemoryAddressTranslator::new(FRAME_SIZE);

        let allocated_frame_identifiers = boot_info
            .memory_regions
            .iter()
            .filter(|region| !matches!(region.kind, MemoryRegionKind::Usable))
            .flat_map(|region| {
                let start = frame_identifier_address_translator
                    .translate_memory_address(PhysicalMemoryAddress::from(region.start));
                let end = frame_identifier_address_translator
                    .translate_memory_address(PhysicalMemoryAddress::from(region.end));

                start..end
            })
            .filter(|identifier| {
                frame_identifier_address_translator
                    .translate_frame_identifier::<PhysicalMemoryAddressPerspective>(*identifier)
                    .as_u64()
                    < highest_usable_address
            })
            // `linker.ld` links the application-processor trampoline (`.smp_sections`) at a
            // fixed low physical address, overwritten in place at SMP bring-up time (see
            // `smp::start_application_processors`). That frame has to be carved out here, before
            // the heap-backing loop below claims the lowest free frames for the heap: otherwise
            // relocating the trampoline later clobbers live heap data sitting at the same frame.
            .chain(once(frame_identifier_address_translator.translate_memory_address(
                PhysicalMemoryAddress::from(unsafe {
                    crate::common::linker_value(&crate::platform::x86_64::smp::SMP_SECTIONS_START)
                } as u64),
            )));

        let frame_manager = FrameManager::new(
            FRAME_SIZE,
            unsafe {
                from_raw_parts_mut(
                    VirtualMemoryAddress::from(physical_memory_offset).cast_mut::<u8>(),
                    highest_usable_address.r#as(),
                )
            },
            allocated_frame_identifiers,
        )?;

        let address_translator = X86_64MemoryManagerAddressTranslator::new(
            physical_memory_offset,
        );

        let mut usable_memory_in_bytes: u64 = 0;

        for region in boot_info.memory_regions.iter() {
            if
                !matches!(
                    region.kind,
                    MemoryRegionKind::Usable
                ) {
                    continue;
                }
            usable_memory_in_bytes += region.end.saturating_sub(region.start);
        }

        /*let mut total_memory_in_bytes: u64 = 0;

        for region in boot_info.memory_regions.iter() {
            total_memory_in_bytes += region.end.saturating_sub(region.start);
        }

        let _ = write!(w, "total memory of all regions: {}\n",total_memory_in_bytes / (1024 * 1024));

        boot_info
            .memory_regions
            .windows(2)
            .filter(|window| window[0].end < window[1].start)
            .for_each(|window| {
                let _ = write!(
                    w,
                    "Hole: {:#x}..{:#x} ({} MiB)\n",
                    window[0].end,
                    window[1].start,
                    (window[1].start - window[0].end) / (1024 * 1024),
                );
            });*/

        let page_table = unsafe {
            address_translator
                .translate_physical_memory_address(PhysicalMemoryAddress::from(
                    Cr3::read().0.start_address().as_u64(),
                ))
                .cast_mut::<PageTable>()
                .as_mut()
                .unwrap()
        };

        let mut page_table_mapper = unsafe {
            OffsetPageTable::new(
                page_table,
                VirtAddr::new(physical_memory_offset.as_usize().r#as()),
            )
        };

        let mut this = Self {
            physical_memory_offset,
            physical_memory_size_in_bytes,
            managed_memory_size: highest_usable_address,
            usable_memory_in_bytes,
            kernel_image_length: boot_info.kernel_len,
            kernel_image_start_virtual_memory_address:
                physical_memory_offset + VirtualMemoryAddress::from(boot_info.kernel_addr),
            frame_manager,
            heap_allocator_reference: None,
        };

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

        for (_i, heap_frame_identifier) in this.frame_manager
            .unallocated_frame_identifiers()
            .take(frame_count)
            .enumerate()
        {
            let physical_address = this.frame_manager.translate_frame_identifier(heap_frame_identifier);

            this.frame_manager()
                .allocate_frames(once(heap_frame_identifier))
                .unwrap();

            unsafe {
                page_table_mapper
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

            current_address += this.frame_manager.frame_byte_size().r#as();
        }

        let allocator = LinkedListHeapMemoryAllocator::new(unsafe {
            LockedHeap::new(
                VirtualMemoryAddress::<DefaultMemoryAddressTransformer>::from(start_address)
                    .cast_mut(),
                frame_count * this.frame_manager.frame_byte_size(),
            )
        });

        let reference = SelfContainedHeapAllocator::new(allocator)?;

        this.heap_allocator_reference = Some(ErasedAllocator::new(reference));

        Ok(this)
    }

    #[allow(dead_code)]
    fn physical_memory(&self) -> &'static mut [u8] {
        unsafe {
            from_raw_parts_mut(
                self.physical_memory_offset.cast_mut::<u8>(),
                self.physical_memory_size_in_bytes.r#as(),
            )
        }
    }

    pub fn physical_memory_size_in_bytes(&self) -> u64 {
        self.physical_memory_size_in_bytes
    }

    pub fn usable_memory_in_bytes(&self) -> u64 {
        self.usable_memory_in_bytes
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
        unsafe {
            FrameManager::new_initialized(
                FRAME_SIZE,
                from_raw_parts_mut(
                    self.physical_memory_offset.cast_mut::<u8>(),
                    self.managed_memory_size.r#as(),
                ),
            )
        }
    }

    fn active_page_table(&self) -> &'static mut PageTable {
        let pointer = self
            .translate_physical_memory_address(PhysicalMemoryAddress::from(
                Cr3::read().0.start_address().as_u64(),
            ))
            .as_u64();

        unsafe { &mut *(pointer as *mut PageTable) }
    }

    pub(crate) fn page_table_mapper(&self) -> OffsetPageTable<'static> {
        unsafe {
            OffsetPageTable::new(
                self.active_page_table(),
                VirtAddr::new(self.physical_memory_offset.as_usize().r#as()),
            )
        }
    }

    pub fn allocate_user_stack(&self) -> Option<VirtualMemoryRegion> {
        let stack_frame_count = 8;
        let stack_size = stack_frame_count * FRAME_SIZE;

        let mut first_address = 0;
        let mut mapper = self.page_table_mapper();

        for stack_frame_identifier in self.frame_manager().allocate_window(8).unwrap() {
            let stack_address = self
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

        Some(VirtualMemoryRegion::new(first_address, stack_size))
    }

    pub fn allocate_stack(&self) -> Option<VirtualMemoryRegion> {
        let stack_frame_count = 8;
        let stack_size = stack_frame_count * FRAME_SIZE;

        let mut first_address = 0;
        let mut mapper = self.page_table_mapper();

        for stack_frame_identifier in self.frame_manager().allocate_window(8).unwrap() {
            let stack_address = self
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

        Some(VirtualMemoryRegion::new(first_address, stack_size))
    }

    pub fn heap_allocator(&self) -> ErasedAllocator {
        self.heap_allocator_reference
            .clone()
            .expect("heap allocator to be initialized")
    }
}

use ztd::Constructor;

#[derive(Constructor)]
pub struct KernelMemoryManagerFrameAllocator<'a> {
    memory_manager: &'a X86_64MemoryManager,
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
    memory_manager: &'a X86_64MemoryManager,
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

use acpi::{AcpiHandler, PhysicalMapping};
use core::ptr::NonNull;

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Constructor, Clone)]
pub struct KernelMemoryManagerAcpiHandler<'a> {
    memory_manager: &'a X86_64MemoryManager,
}

impl<'a> AcpiHandler for KernelMemoryManagerAcpiHandler<'a> {
    unsafe fn map_physical_region<T>(
        &self,
        physical_address: usize,
        size: usize,
    ) -> PhysicalMapping<Self, T> {
        unsafe {
            let address = self
                .memory_manager
                .translate_physical_memory_address(PhysicalMemoryAddress::from(physical_address));

            PhysicalMapping::new(
                physical_address,
                NonNull::new(address.cast_mut::<T>()).unwrap(),
                size,
                size,
                self.clone(),
            )
        }
    }

    fn unmap_physical_region<T>(_region: &PhysicalMapping<Self, T>) {}
}

use crate::common::memory::allocator::ErasedAllocator;

