use crate::future::runtime::FutureRuntimeCommonBounds;

////////////////////////////////////////////////////////////////////////////////////////////////////

pub trait FutureRuntimeYielder: FutureRuntimeCommonBounds {
    fn r#yield(&self);
}
