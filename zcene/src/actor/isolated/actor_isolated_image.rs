use crate::actor::isolated::{ActorIsolatedImageError, ActorIsolatedImageSegment};
use xmas_elf::header::{Class, Machine, Type as ElfType};
use xmas_elf::program::Type as ProgramType;
use xmas_elf::ElfFile;

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Clone, Copy, Debug)]
pub struct ActorIsolatedImage {
    bytes: &'static [u8],
    base: Option<u64>,
    entry_offset: usize,
    segments: [ActorIsolatedImageSegment; Self::MAXIMUM_SEGMENTS],
    segment_count: usize,
}

impl ActorIsolatedImage {
    pub const MAXIMUM_SEGMENTS: usize = 8;

    /// The ELF parser reads its structures in place, so the bytes have to be aligned. Use
    /// `include_actor_isolated_image!` to embed a file.
    pub const ELF_ALIGNMENT: usize = 8;

    /// A position independent block of code that is copied to the start of the address space,
    /// readable and executable but not writable, and entered at `entry_offset`.
    pub const fn new(bytes: &'static [u8], entry_offset: usize) -> Self {
        let mut segments = [ActorIsolatedImageSegment::EMPTY; Self::MAXIMUM_SEGMENTS];
        segments[0] = ActorIsolatedImageSegment::new(0, bytes.len(), 0, bytes.len(), false);

        Self {
            bytes,
            base: None,
            entry_offset,
            segments,
            segment_count: 1,
        }
    }

    /// A statically linked x86_64 ELF executable. Its loadable segments are placed in the address
    /// space at their virtual addresses (which have to start at the address space's base), the
    /// ones that are not writable first, and it is entered at the ELF entry point.
    pub fn from_elf(bytes: &'static [u8]) -> Result<Self, ActorIsolatedImageError> {
        if bytes.as_ptr() as usize % Self::ELF_ALIGNMENT != 0 {
            return Err(ActorIsolatedImageError::Misaligned);
        }

        let elf = ElfFile::new(bytes).map_err(|_| ActorIsolatedImageError::NotElf)?;

        if elf.header.pt1.class() != Class::SixtyFour
            || elf.header.pt2.type_().as_type() != ElfType::Executable
            || elf.header.pt2.machine().as_machine() != Machine::X86_64
        {
            return Err(ActorIsolatedImageError::Unsupported);
        }

        let mut base = u64::MAX;

        for header in elf.program_iter() {
            if header.get_type() == Ok(ProgramType::Load) {
                base = base.min(header.virtual_addr());
            }
        }

        if base == u64::MAX {
            return Err(ActorIsolatedImageError::Malformed);
        }

        let mut segments = [ActorIsolatedImageSegment::EMPTY; Self::MAXIMUM_SEGMENTS];
        let mut segment_count = 0;

        for header in elf.program_iter() {
            if header.get_type() != Ok(ProgramType::Load) {
                continue;
            }

            let flags = header.flags();

            if flags.is_write() && flags.is_execute() {
                return Err(ActorIsolatedImageError::WritableAndExecutable);
            }

            let (file_offset, file_length, memory_length) =
                (header.offset(), header.file_size(), header.mem_size());

            if file_length > memory_length
                || file_offset.checked_add(file_length).is_none_or(|end| end > bytes.len() as u64)
            {
                return Err(ActorIsolatedImageError::Malformed);
            }

            let segment = segments
                .get_mut(segment_count)
                .ok_or(ActorIsolatedImageError::TooManySegments)?;

            *segment = ActorIsolatedImageSegment::new(
                file_offset as usize,
                file_length as usize,
                usize::try_from(header.virtual_addr() - base)
                    .map_err(|_| ActorIsolatedImageError::Malformed)?,
                usize::try_from(memory_length).map_err(|_| ActorIsolatedImageError::Malformed)?,
                flags.is_write(),
            );

            segment_count += 1;
        }

        let entry_offset = elf
            .header
            .pt2
            .entry_point()
            .checked_sub(base)
            .and_then(|offset| usize::try_from(offset).ok())
            .ok_or(ActorIsolatedImageError::Malformed)?;

        Ok(Self {
            bytes,
            base: Some(base),
            entry_offset,
            segments,
            segment_count,
        })
    }

    pub const fn bytes(&self) -> &'static [u8] {
        self.bytes
    }

    pub const fn base(&self) -> Option<u64> {
        self.base
    }

    pub const fn entry_offset(&self) -> usize {
        self.entry_offset
    }

    pub fn segments(&self) -> &[ActorIsolatedImageSegment] {
        &self.segments[..self.segment_count]
    }
}
