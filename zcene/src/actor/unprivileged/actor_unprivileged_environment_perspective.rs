use zcene_core::actor::ActorMessage;

////////////////////////////////////////////////////////////////////////////////////////////////////

pub trait ActorUnprivilegedEnvironmentPerspective: Send + Sync + 'static {
    type Handle<M>: Clone + Send + Sync + 'static
    where
        M: ActorMessage;
}
