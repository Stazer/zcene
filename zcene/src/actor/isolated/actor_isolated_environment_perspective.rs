use zcene_core::actor::ActorMessage;

////////////////////////////////////////////////////////////////////////////////////////////////////

pub trait ActorIsolatedEnvironmentPerspective: Send + Sync + 'static {
    type Handle<M>: Clone + Send + Sync + 'static
    where
        M: ActorMessage;
}
