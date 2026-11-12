#[derive(Clone, Copy, Debug)]
pub struct ActorIsolatedImageSegment {
    file_offset: usize,
    file_length: usize,
    memory_offset: usize,
    memory_length: usize,
    writable: bool,
}

impl ActorIsolatedImageSegment {
    pub const EMPTY: Self = Self {
        file_offset: 0,
        file_length: 0,
        memory_offset: 0,
        memory_length: 0,
        writable: false,
    };

    pub const fn new(
        file_offset: usize,
        file_length: usize,
        memory_offset: usize,
        memory_length: usize,
        writable: bool,
    ) -> Self {
        Self {
            file_offset,
            file_length,
            memory_offset,
            memory_length,
            writable,
        }
    }

    pub const fn file_offset(&self) -> usize {
        self.file_offset
    }

    pub const fn file_length(&self) -> usize {
        self.file_length
    }

    pub const fn memory_offset(&self) -> usize {
        self.memory_offset
    }

    pub const fn memory_length(&self) -> usize {
        self.memory_length
    }

    pub const fn writable(&self) -> bool {
        self.writable
    }
}
