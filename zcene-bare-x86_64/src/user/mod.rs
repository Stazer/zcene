extern "C" {
    pub static USER_SECTIONS_START: usize;
    pub static USER_SECTIONS_END: usize;
    pub static USER_SECTIONS_SIZE: usize;
}

use zcene_core::actor::{
    Actor, ActorCreateError, ActorDestroyError, ActorHandleError, ActorHandler,
};

pub struct MainActor;

impl<H> Actor<H> for MainActor
where
    H: ActorHandler,
{
    type Message = ();

    #[link_section = ".user_sections"]
    async fn create(&mut self, _context: H::CreateContext) -> Result<(), ActorCreateError> {
        Ok(())
    }

    #[link_section = ".user_sections"]
    async fn handle(
        &mut self,
        _context: H::HandleContext<Self::Message>,
    ) -> Result<(), ActorHandleError> {
        Ok(())
    }

    #[link_section = ".user_sections"]
    async fn destroy(self, _context: H::DestroyContext) -> Result<(), ActorDestroyError> {
        Ok(())
    }
}
