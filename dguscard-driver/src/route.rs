use dguscard::response::Content;
use dguscard::{Address, response};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[non_exhaustive]
pub enum RouteError {
    /// No route is registered for this address and word length.
    Unhandled(Address, u8),
    /// A route matched, but its payload failed to deserialize.
    Deserialize(Address, response::Error),
}

pub trait Router {
    type Context;

    /// Routes one unsolicited data frame.
    fn route(&mut self, ctx: &mut Self::Context, content: Content<'_>) -> Result<(), RouteError>;
}
