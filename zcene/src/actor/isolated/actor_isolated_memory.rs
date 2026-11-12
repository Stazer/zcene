use crate::actor::isolated::{ActorIsolatedImage, ActorIsolatedMessage};
use crate::platform::{
    ActivePlatform, Platform, UnprivilegedAddressSpace, UnprivilegedAddressSpaceSpecification,
};
use core::alloc::{AllocError, Layout};
use core::marker::PhantomData;
use core::mem::{align_of, size_of};
use zcene_core::actor::ActorSpawnError;

////////////////////////////////////////////////////////////////////////////////////////////////////

const FRAME_SIZE: usize = 4096;

/// The address space of one isolated actor and where things are in it:
///
/// ```text
/// user_base                                                         user_base + size
/// | image (r-x, then rw-) | extra data (rw-) | message slot | (padding) | stack (rw-) ->grows down |
/// ```
///
/// The extra data region starts right after the image, rounded up to whole frames.
pub struct ActorIsolatedMemory<M>
where
    M: ActorIsolatedMessage,
{
    address_space: <ActivePlatform as Platform>::UnprivilegedAddressSpace,
    entry: u64,
    message_offset: usize,
    marker: PhantomData<fn() -> M>,
}

impl<M> ActorIsolatedMemory<M>
where
    M: ActorIsolatedMessage,
{
    pub const DATA_SIZE: usize = 16 * 1024;
    pub const STACK_SIZE: usize = 32 * 1024;

    pub const MAXIMUM_IMAGE_SIZE: usize = 16 * 1024 * 1024;

    pub fn new(platform: &ActivePlatform, image: &ActorIsolatedImage) -> Result<Self, ActorSpawnError> {
        let segments = image.segments();

        let mut footprint = 0usize;
        let mut read_only_end = 0usize;

        for segment in segments {
            let end = segment
                .memory_offset()
                .checked_add(segment.memory_length())
                .ok_or(ActorSpawnError::Unknown)?;

            footprint = footprint.max(end);

            if !segment.writable() {
                read_only_end = read_only_end.max(end);
            }
        }

        let executable_length = read_only_end.next_multiple_of(FRAME_SIZE);
        let footprint = footprint.next_multiple_of(FRAME_SIZE);

        let entry_in_code = segments.iter().any(|segment| {
            !segment.writable()
                && image.entry_offset() >= segment.memory_offset()
                && image.entry_offset() - segment.memory_offset() < segment.memory_length()
        });

        if segments.is_empty()
            || !entry_in_code
            || footprint > Self::MAXIMUM_IMAGE_SIZE
            || align_of::<M>() > FRAME_SIZE
            || segments
                .iter()
                .any(|segment| segment.writable() && segment.memory_offset() < executable_length)
        {
            return Err(ActorSpawnError::Unknown);
        }

        let message_offset = (footprint + Self::DATA_SIZE).next_multiple_of(align_of::<M>());
        let stack_offset = (message_offset + size_of::<M>()).next_multiple_of(FRAME_SIZE);
        let size = stack_offset + Self::STACK_SIZE;

        let address_space = platform
            .create_unprivileged_address_space(&UnprivilegedAddressSpaceSpecification {
                size,
                executable_length,
                required_base: image.base(),
            })
            .filter(|address_space| address_space.size() >= size)
            .ok_or(AllocError)?;

        for segment in segments {
            // SAFETY: the segment lies within the image (checked when it was created) and the
            // address space is at least `size` bytes, which is more than the footprint.
            unsafe {
                core::ptr::copy_nonoverlapping(
                    image.bytes().as_ptr().add(segment.file_offset()),
                    address_space.kernel_base().add(segment.memory_offset()),
                    segment.file_length(),
                );
            }
        }

        Ok(Self {
            entry: address_space.user_base() + image.entry_offset() as u64,
            address_space,
            message_offset,
            marker: PhantomData,
        })
    }

    pub fn address_space(&self) -> &<ActivePlatform as Platform>::UnprivilegedAddressSpace {
        &self.address_space
    }

    pub fn entry(&self) -> u64 {
        self.entry
    }

    pub fn stack_top(&self) -> u64 {
        self.address_space.user_base() + self.address_space.size() as u64
    }

    /// Moves `message` into the message slot and returns its address in the address space.
    pub fn store_message(&mut self, message: M) -> u64 {
        // SAFETY: the slot is inside the address space and aligned, see `new`.
        unsafe {
            self.address_space
                .kernel_base()
                .add(self.message_offset)
                .cast::<M>()
                .write(message);
        }

        self.address_space.user_base() + self.message_offset as u64
    }

    pub fn contains(&self, user_address: u64, layout: Layout) -> bool {
        if layout.size() == 0 {
            return true;
        }

        let Some(offset) = user_address.checked_sub(self.address_space.user_base()) else {
            return false;
        };

        let offset = offset as usize;

        offset % layout.align() == 0
            && offset
                .checked_add(layout.size())
                .is_some_and(|end| end <= self.address_space.size())
    }

    /// The kernel's own pointer to the memory behind `user_address`. Only valid after `contains`.
    pub fn kernel_address(&self, user_address: u64) -> usize {
        self.address_space.kernel_base() as usize
            + (user_address - self.address_space.user_base()) as usize
    }
}
