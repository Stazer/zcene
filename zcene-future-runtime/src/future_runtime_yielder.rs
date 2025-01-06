pub trait FutureRuntimeYielder: Send + Sync + 'static {
    fn r#yield(&self);
}
