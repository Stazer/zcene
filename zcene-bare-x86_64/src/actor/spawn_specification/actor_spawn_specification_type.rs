use crate::actor::{
    ActorInlineSpawnSpecification, ActorPrivilegedSpawnSpecification,
    ActorUnprivilegedSpawnSpecification,
};
use ztd::From;

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(From)]
#[From(all)]
pub enum ActorSpawnSpecificationType {
    Inline(ActorInlineSpawnSpecification),
    Privileged(ActorPrivilegedSpawnSpecification),
    Unprivileged(ActorUnprivilegedSpawnSpecification),
}
