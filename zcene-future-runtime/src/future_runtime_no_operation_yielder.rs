use crate::FutureRuntimeYielder;

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Default)]
pub struct FutureRuntimeNoOperationYielder;

impl FutureRuntimeYielder for FutureRuntimeNoOperationYielder {
    fn r#yield(&self) {}
}
