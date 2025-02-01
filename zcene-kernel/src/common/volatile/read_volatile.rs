use crate::common::volatile::{Volatile, VolatileReadAccessMode};

////////////////////////////////////////////////////////////////////////////////////////////////////

pub type ReadVolatile<T> = Volatile<T, VolatileReadAccessMode>;
