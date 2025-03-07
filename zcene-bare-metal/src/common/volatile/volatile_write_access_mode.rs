use crate::common::volatile::{VolatileAccessMode, VolatileWritingAccessMode};

////////////////////////////////////////////////////////////////////////////////////////////////////

pub struct VolatileWriteAccessMode;

impl VolatileAccessMode for VolatileWriteAccessMode {}

impl VolatileWritingAccessMode for VolatileWriteAccessMode {}
