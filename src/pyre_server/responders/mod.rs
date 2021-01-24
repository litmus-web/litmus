pub mod sender;
pub mod receiver;

/// The payload that gets sent to the receiver half of the channel.
pub type Payload = (bool, Vec<u8>);
