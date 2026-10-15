use crate::common::volatile::{
    VolatileAccessMode, VolatileReadingAccessMode, VolatileWritingAccessMode,
};

////////////////////////////////////////////////////////////////////////////////////////////////////

pub struct VolatileReadWriteAccessMode;

impl VolatileAccessMode for VolatileReadWriteAccessMode {}

impl VolatileReadingAccessMode for VolatileReadWriteAccessMode {}

impl VolatileWritingAccessMode for VolatileReadWriteAccessMode {}
