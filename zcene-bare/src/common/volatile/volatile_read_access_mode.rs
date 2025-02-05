use crate::common::volatile::{VolatileAccessMode, VolatileReadingAccessMode};

////////////////////////////////////////////////////////////////////////////////////////////////////

pub struct VolatileReadAccessMode;

impl VolatileAccessMode for VolatileReadAccessMode {}

impl VolatileReadingAccessMode for VolatileReadAccessMode {}
