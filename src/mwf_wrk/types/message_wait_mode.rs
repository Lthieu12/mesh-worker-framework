/// Type of message completion being awaited.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageWaitMode {
    /// Wait until the receiver finishes processing this message.
    WaitThisMessage,

    /// Wait until the receiver completes the message flow by calling `message_flow_end()`.
    WaitThisMessageFlow,
}
