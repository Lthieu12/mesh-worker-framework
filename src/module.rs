use crate::message::FrameworkMessage;
//use crate::error::FrameworkError;

pub type ModuleID = String;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ModuleState {
    Created,
    Initialized,
}

// TODO:
// Revisit async trait design when runtime architecture is finalized.
#[allow(async_fn_in_trait)]
pub trait Module {

    async fn on_init(
        &mut self,
    );

    async fn on_message(
        &mut self,
        message: FrameworkMessage,
    );

    async fn on_close(
        &mut self,
    );
}