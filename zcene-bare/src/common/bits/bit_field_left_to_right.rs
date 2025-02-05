use crate::common::bits::{BitFieldEndianess, BitFieldSignificance};

////////////////////////////////////////////////////////////////////////////////////////////////////

pub struct BitFieldLeftToRight;

impl BitFieldSignificance for BitFieldLeftToRight {}
impl BitFieldEndianess for BitFieldLeftToRight {}
