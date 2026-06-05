    /*fn initialize_smp(&mut self) -> Result<(), InitializeKernelError> {
        let smp_frame_identifier = self
            .memory_manager()
            .frame_manager()
            .unallocated_frame_identifiers()
            .next()
            .unwrap();

        self.memory_manager
            .frame_manager()
            .allocate_frames(once(smp_frame_identifier))?;

        let smp_memory_address = self
            .memory_manager()
            .frame_manager()
            .translate_frame_identifier(smp_frame_identifier);

        use crate::architecture::smp::SMP_SECTIONS_START;
        use crate::architecture::smp::{SMP_HEADER, SMP_SECTIONS_SIZE};
        use core::slice::{from_raw_parts, from_raw_parts_mut};

        let smp_sections_start = unsafe { linker_value(&SMP_SECTIONS_START) };
        let smp_sections_size = unsafe { linker_value(&SMP_SECTIONS_SIZE) };

        let smp_section_from =
            unsafe { from_raw_parts((smp_sections_start) as *const u8, smp_sections_size) };

        let smp_section_to = unsafe {
            from_raw_parts_mut(
                self.memory_manager
                    .translate_physical_memory_address(PhysicalMemoryAddress::from(
                        smp_memory_address,
                    ))
                    .cast_mut::<u8>(),
                //(self.memory_manager.physical_memory_offset() + smp_memory_address.as_u64()) as *mut u8,
                smp_sections_size,
            )
        };

        smp_section_to.copy_from_slice(smp_section_from);

        let mut mapper = self.memory_manager().page_table_mapper();

        let smp_page = Page::<Size4KiB>::containing_address(VirtAddr::new(
            smp_sections_start.try_into().unwrap(),
        ));

        unsafe {
            mapper.unmap(smp_page).expect("hello").1.ignore();
            mapper
                .map_to(
                    smp_page,
                    PhysFrame::from_start_address(PhysAddr::new(0)).unwrap(),
                    PageTableFlags::PRESENT | PageTableFlags::WRITABLE,
                    &mut EmptyFrameAllocator,
                )
                .expect("dooooO!")
                .flush();
        }

        let stack_frame_count = 4;
        let stack_size = stack_frame_count * FRAME_SIZE;
        let total_stack_size = (self.cores - 1) * stack_size;

        for stack_frame_identifier in self
            .memory_manager()
            .frame_manager()
            .allocate_window((self.cores - 1) * stack_frame_count)?
        {
            let stack_address = self
                .memory_manager()
                .frame_manager()
                .translate_frame_identifier(stack_frame_identifier);

            use x86_64::instructions::tables::sgdt;

            let page = Page::<Size4KiB>::containing_address(VirtAddr::new(
                (stack_address.as_usize()).try_into().unwrap(),
            ));

            unsafe {
                mapper
                    .map_to(
                        page,
                        PhysFrame::from_start_address(PhysAddr::new(
                            stack_address.as_usize().try_into().unwrap(),
                        ))
                        .unwrap(),
                        PageTableFlags::PRESENT | PageTableFlags::WRITABLE,
                        &mut EmptyFrameAllocator,
                    )
                    .expect("Hello World")
                    .flush();
            }

            unsafe {
                if SMP_HEADER.stack_start == 0 {
                    SMP_HEADER.stack_start = page.start_address().as_u64() + stack_size as u64;
                    SMP_HEADER.stack_size = stack_size as _;
                    SMP_HEADER.page_table_start =
                        Cr3::read().0.start_address().as_u64().try_into().expect("");
                    SMP_HEADER.gdt64_pointer = sgdt();
                }
            }
        }

        Ok(())
    }

    fn boot_application_processors(&mut self, mut local_apic: LocalApic) {
        unsafe {
            local_apic.send_init_ipi_all();

            for i in 0..100000000 {
                core::hint::black_box(());
                x86_64::instructions::nop();
            }

            local_apic.send_sipi_all(
                (crate::architecture::smp::smp_real_mode_entry as u64)
                    .try_into()
                    .unwrap(),
            );
        }
    }*/


    /*fn initialize_cores(&mut self) {
        let cpu_id = CpuId::new();

        self.cores = 0;

        for extended_topology_info in cpu_id.get_extended_topology_info().into_iter().flatten() {
            self.cores += match extended_topology_info.level_type() {
                TopologyType::Core => extended_topology_info.processors() as usize,
                _ => 0,
            };
        }
    }*/
