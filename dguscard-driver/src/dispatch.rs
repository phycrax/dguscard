use dguscard::response::Content;
use dguscard::{Address, response};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[non_exhaustive]
pub enum DispatchError {
    /// No route is registered for this address and word length.
    Unhandled(Address, u8),
    /// A route matched, but its payload failed to deserialize.
    Deserialize(Address, response::Error),
}

pub trait Dispatcher {
    /// Handles one unsolicited data frame.
    fn dispatch(&mut self, content: Content<'_>) -> Result<(), DispatchError>;
}
