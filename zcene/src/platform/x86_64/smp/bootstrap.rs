use crate::common::linker_value;
use crate::common::memory::address::PhysicalMemoryAddress;
use crate::common::memory::allocator::FlatBitmapFrameAllocatorAllocationError;
use crate::driver::xapic::{XApic, XApicRegisters};
use crate::platform::x86_64::smp::{SMP_HEADER, SMP_SECTIONS_SIZE, SMP_SECTIONS_START, smp_real_mode_entry};
use crate::platform::x86_64::{FRAME_SIZE, KernelMemoryManagerAcpiHandler, KernelMemoryManagerFrameAllocator};
use crate::platform::x86_64::{X86_64MemoryManager, X86_64Platform};
use acpi::AcpiTables;
use acpi::madt::{Madt, MadtEntry};
use alloc::vec::Vec;
use core::fmt::Write;
use core::iter::once;
use core::slice::{from_raw_parts, from_raw_parts_mut};
use core::sync::atomic::{AtomicBool, Ordering};
use x86_64::instructions::tables::sgdt;
use x86_64::registers::control::{Cr3, Cr4};
use x86_64::structures::paging::{Mapper, Page, PageTableFlags, PhysFrame, Size4KiB};
use x86_64::{PhysAddr, VirtAddr};

////////////////////////////////////////////////////////////////////////////////////////////////////

/// Set right before the INIT-SIPI-SIPI sequence is sent, once `ACTOR_ROOT_ENVIRONMENT` is
/// guaranteed to be initialized. Application processors spin on this before touching it, since
/// they otherwise have no way to know whether the bootstrap processor has finished setting it up.
static AP_READY: AtomicBool = AtomicBool::new(false);

// Plain cycle-counted busy-waits, not `Timer`-based: at this point in boot `Timer` may still be
// the software `AtomicTimer` fallback (no HPET found), whose `duration_between` is always zero,
// which would turn a real delay into an infinite loop. These counts are deliberately generous
// approximations of the Intel-recommended ~10ms / ~200us INIT-SIPI-SIPI gaps.
const INIT_IPI_DELAY_ITERATIONS: u64 = 50_000_000;
const STARTUP_IPI_RETRY_DELAY_ITERATIONS: u64 = 1_000_000;

////////////////////////////////////////////////////////////////////////////////////////////////////

/// Brings up every other logical core found in the ACPI MADT using the standard INIT-SIPI-SIPI
/// sequence. Best-effort: on any failure (no ACPI tables, no free low frame for the trampoline,
/// ...) this logs and leaves the machine running on the bootstrap processor alone.
pub fn start_application_processors(platform: &X86_64Platform) {
    let application_processor_count = application_processor_count(platform);

    if application_processor_count == 0 {
        return;
    }

    let memory = platform.memory();

    let Some(trampoline_physical_address) = (unsafe { relocate_trampoline(memory) }) else {
        let _ = write!(
            platform.logger().writer(),
            "smp: could not relocate the application processor trampoline, staying single-core\n",
        );

        return;
    };

    if prepare_application_processor_stacks(memory, application_processor_count).is_none() {
        let _ = write!(
            platform.logger().writer(),
            "smp: could not prepare application processor stacks, staying single-core\n",
        );

        return;
    }

    let Some(xapic_base) = platform.interrupt_manager().bootstrap_xapic_base() else {
        let _ = write!(
            platform.logger().writer(),
            "smp: no local APIC available, staying single-core\n",
        );

        return;
    };

    // Everything an application processor could read (environment, GDT/IDT, trampoline, stacks)
    // is in place now, so it is safe to let them past their spin wait.
    AP_READY.store(true, Ordering::Release);

    let vector = (trampoline_physical_address.as_u64() / FRAME_SIZE as u64) as u8;

    unsafe {
        let mut xapic = XApic::new((xapic_base as *mut XApicRegisters).as_mut().unwrap());

        xapic.send_init_ipi_all_excluding_self();
        spin_delay_cycles(INIT_IPI_DELAY_ITERATIONS);

        // Sent twice, with a short delay in between: some real hardware drops the first SIPI.
        xapic.send_startup_ipi_all_excluding_self(vector);
        spin_delay_cycles(STARTUP_IPI_RETRY_DELAY_ITERATIONS);
        xapic.send_startup_ipi_all_excluding_self(vector);
    }
}

/// Called via `jmp` from `long_mode.asm` on every application processor, with paging, a private
/// stack and the bootstrap GDT already in place (see `long_mode.asm` / `protected_mode.asm`).
#[unsafe(no_mangle)]
pub extern "C" fn application_processor_entry_point() -> ! {
    while !AP_READY.load(Ordering::Acquire) {
        core::hint::spin_loop();
    }

    let environment = crate::actor::ActorRootEnvironment::get();

    // The IDT is shared across cores; each core still has to point its own IDTR at it, or any
    // exception on this core triple-faults the machine instead of being handled.
    environment
        .platform()
        .interrupt_manager()
        .load_bootstrap_descriptor_table();

    environment.future_runtime().run();

    loop {
        core::hint::spin_loop();
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////

fn application_processor_count(platform: &X86_64Platform) -> usize {
    let Some(rsdp_address) = platform.rsdp_address() else {
        return 0;
    };

    let handler = KernelMemoryManagerAcpiHandler::new(platform.memory());

    let Ok(tables) = (unsafe { AcpiTables::from_rsdp(handler, rsdp_address as usize) }) else {
        return 0;
    };

    // Deliberately not `platform_info()`/`platform_info_in()`: those collect the processor list
    // into heap-allocated `ManagedSlice`s. Counting the MADT entries directly needs no
    // allocation at all, which matters this early in boot (see `relocate_trampoline`'s doc
    // comment for why a stray allocation here is not something to risk).
    let Ok(madt) = tables.find_table::<Madt>() else {
        return 0;
    };

    let mut enabled_processor_count = 0usize;

    for entry in madt.get().entries() {
        let flags = match entry {
            MadtEntry::LocalApic(entry) => entry.flags,
            MadtEntry::LocalX2Apic(entry) => entry.flags,
            _ => continue,
        };

        if flags & 1 != 0 {
            enabled_processor_count += 1;
        }
    }

    // One of these is the BSP, already running.
    enabled_processor_count.saturating_sub(1)
}

/// Copies the `.smp_sections` trampoline (linked at a fixed low address, see `linker.ld`) into
/// the frame reserved for it (see `X86_64MemoryManager::new`) and identity-maps that address to
/// it in the live kernel page table, so the real-mode startup code and the post-paging
/// long-mode code are the same bytes at the same address either way.
///
/// # Safety
///
/// Must only run once, before any application processor is started, since it mutates the live
/// kernel page table.
unsafe fn relocate_trampoline(memory: &X86_64MemoryManager) -> Option<PhysicalMemoryAddress> {
    let smp_sections_start = unsafe { linker_value(&SMP_SECTIONS_START) };
    let smp_sections_size = unsafe { linker_value(&SMP_SECTIONS_SIZE) };

    // `smp_real_mode_entry` is the first byte of `.smp_sections` (see `linker.ld` and
    // `real_mode.asm`), so this must always hold. Asserting it (rather than just a
    // `debug_assert!`) also keeps a real reference to the symbol around in release builds:
    // without one, nothing makes the real/protected/long-mode trampoline reachable from Rust,
    // and the linker's default `--gc-sections` silently drops all three `.asm` sections.
    assert_eq!(
        smp_sections_start, smp_real_mode_entry as *const () as usize,
        "smp: .smp_sections is not linked where smp_real_mode_entry expects it",
    );

    // `linker.ld` links `.smp_sections` at a fixed low address (currently 0x0), and the
    // real/protected-mode stages run with paging off, so the trampoline's internal far jumps
    // only land correctly if the bytes physically live at exactly that address — there is no
    // freedom to place them at some other free frame below 1 MiB.
    let smp_physical_address = PhysicalMemoryAddress::from(smp_sections_start as u64);

    if smp_physical_address.as_u64() >= 0x100000 {
        return None;
    }

    let smp_frame_identifier = memory
        .frame_allocator()
        .translate_memory_address(smp_physical_address);

    // `X86_64MemoryManager::new` carves this exact frame out before anything else (in
    // particular, before the heap claims the lowest free frames for its own backing memory), so
    // it is expected to already show up as allocated here; only a genuine allocator problem
    // (out of range, bitmap busy, ...) should abort bring-up.
    match memory.frame_allocator().allocate_frames(once(smp_frame_identifier)) {
        Ok(()) | Err(FlatBitmapFrameAllocatorAllocationError::Allocated(_)) => {}
        Err(_) => return None,
    }

    let smp_section_from = unsafe { from_raw_parts(smp_sections_start as *const u8, smp_sections_size) };

    let smp_section_to = unsafe {
        from_raw_parts_mut(
            memory
                .translate_physical_memory_address(smp_physical_address)
                .cast_mut::<u8>(),
            smp_sections_size,
        )
    };

    smp_section_to.copy_from_slice(smp_section_from);

    let mut mapper = memory.page_table_mapper();

    let smp_page = Page::<Size4KiB>::containing_address(VirtAddr::new(smp_sections_start as u64));

    unsafe {
        if let Ok((_, flush)) = mapper.unmap(smp_page) {
            flush.ignore();
        }

        mapper
            .map_to(
                smp_page,
                PhysFrame::from_start_address(PhysAddr::new(smp_physical_address.as_u64())).ok()?,
                PageTableFlags::PRESENT | PageTableFlags::WRITABLE,
                &mut KernelMemoryManagerFrameAllocator::new(memory),
            )
            .ok()?
            .flush();
    }

    Some(smp_physical_address)
}

/// Allocates one contiguous block of stacks (one per application processor) and tells
/// `long_mode.asm` how to find them: `SMP_HEADER.stack_start` is the top of the *first* core's
/// stack, and each application processor computes its own as `stack_start + (apic_id - 1) *
/// stack_size`. This assumes APIC IDs `1..=application_processor_count` are exactly the
/// processors being started here, which holds for the simple topologies (e.g. QEMU `-smp`) this
/// kernel currently targets, but is not guaranteed by the ACPI/APIC specs in general.
fn prepare_application_processor_stacks(
    memory: &X86_64MemoryManager,
    application_processor_count: usize,
) -> Option<()> {
    let stack_frame_count = 4;
    let stack_size = stack_frame_count * FRAME_SIZE;

    let mut frame_allocator = memory.frame_allocator();
    let mut stack_frame_identifiers = Vec::new_in(memory.heap_allocator());
    stack_frame_identifiers.extend(
        frame_allocator
            .allocate_window(application_processor_count * stack_frame_count)
            .ok()?,
    );
    drop(frame_allocator);

    let mut mapper = memory.page_table_mapper();

    for stack_frame_identifier in stack_frame_identifiers {
        let stack_address = memory
            .frame_allocator()
            .translate_frame_identifier(stack_frame_identifier);

        let page = Page::<Size4KiB>::containing_address(VirtAddr::new(stack_address.as_u64()));

        unsafe {
            mapper
                .map_to(
                    page,
                    PhysFrame::from_start_address(PhysAddr::new(stack_address.as_u64())).ok()?,
                    PageTableFlags::PRESENT | PageTableFlags::WRITABLE,
                    &mut KernelMemoryManagerFrameAllocator::new(memory),
                )
                .ok()?
                .flush();

            if SMP_HEADER.stack_start == 0 {
                SMP_HEADER.stack_start = page.start_address().as_u64() + stack_size as u64;
                SMP_HEADER.stack_size = stack_size as u64;
                SMP_HEADER.page_table_start =
                    Cr3::read().0.start_address().as_u64().try_into().ok()?;
                SMP_HEADER.gdt64_pointer = sgdt();
                SMP_HEADER.cr4 = Cr4::read_raw();
            }
        }
    }

    Some(())
}

fn spin_delay_cycles(iterations: u64) {
    for _ in 0..iterations {
        core::hint::black_box(());
        x86_64::instructions::nop();
    }
}
