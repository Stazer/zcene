use ztd::{Display, Error, From};

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Clone, Debug, Display, Error, From)]
#[From(all)]
pub enum ActorEnterError {
    Unknown,
}
