use crate::{Address, Content, Error};

/// A routing table mapping `(Address, wlen)` keys to payload handler functions.
///
/// The [`router!`] macro generates an impl of this trait, but it can also be
/// implemented by hand for full control over routing. Either way, it is driven by
/// [`Runner::run`].
///
/// [`router!`]: crate::router
/// [`Runner::run`]: crate::Runner::run
pub trait Router {
    /// Shared state threaded to every handler.
    type Context;

    /// Deserializes `content` into the type expected by the handler mapped to
    /// `(addr, wlen)` and invokes it. A macro-generated impl ignores unmapped
    /// keys, or returns [`Error::Unhandled`] when the table sets
    /// `ignore_unhandled: false`; a hand-written impl decides its own behavior.
    ///
    /// [`Error::Unhandled`]: crate::Error::Unhandled
    fn route(
        &mut self,
        addr: Address,
        wlen: u8,
        content: &mut Content<'_>,
        cx: &mut Self::Context,
    ) -> Result<(), Error>;
}

/// Builds a [`Router`] routing table that maps `(Address, wlen)` frame keys to
/// handler functions.
///
/// # Example
///
/// ```ignore
/// fn temperature_handler(cx: &mut App, val: Temperature) { /* ... */ }
/// fn flags_handler(cx: &mut App, val: Flags) { /* ... */ }
/// fn counter_handler(cx: &mut App, val: Counter) { /* ... */ }
///
/// router! {
///     name: MyRouter,
///     context: App,
///     ignore_unhandled: true,
///     routes: {
///         Word(0x1234), wlen: 2 => temperature_handler,
///         Register(0x02, 0x10), wlen: 1 => flags_handler,
///         Dword(0x8000), wlen: 4 => counter_handler,
///     }
/// }
/// ```
///
/// Each handler is a function `fn(&mut Context, T)` where `T` implements
/// [`serde::Deserialize`]; the macro deserializes the payload into `T` (inferred
/// from the handler signature) before the call. Frames whose `(addr, wlen)` is
/// not in the table are ignored when `ignore_unhandled` is `true`, otherwise they
/// return [`Error::Unhandled`].
///
/// [`Router`]: $crate::Router
/// [`Error::Unhandled`]: $crate::Error::Unhandled
#[macro_export]
macro_rules! router {
    (
        name: $vis:vis $name:ident,
        context: $ctx:ty,
        ignore_unhandled: $ignore:literal,
        routes: { $($routes:tt)* } $(,)?
    ) => {
        $vis struct $name;

        impl $crate::Router for $name {
            type Context = $ctx;

            fn route(
                &mut self,
                addr: $crate::Address,
                wlen: u8,
                content: &mut $crate::Content<'_>,
                cx: &mut $ctx,
            ) -> ::core::result::Result<(), $crate::Error> {
                $crate::router!(@munch [cx, content, addr, wlen, $ignore] [] $($routes)*);
                ::core::result::Result::Ok(())
            }
        }
    };

    // Accumulate one arm per address variant, then recurse on the rest.
    (@munch [$cx:ident, $content:ident, $addr:ident, $wlen:ident, $ignore:literal] [$($arms:tt)*]
        Word($a:literal), wlen: $w:literal => $handler:path $(, $($rest:tt)*)?
    ) => {
        $crate::router!(@munch [$cx, $content, $addr, $wlen, $ignore]
            [$($arms)* ($crate::Address::Word($a), $w) =>
                $crate::router!(@call $cx, $content, $addr, $handler),]
            $($($rest)*)?)
    };
    (@munch [$cx:ident, $content:ident, $addr:ident, $wlen:ident, $ignore:literal] [$($arms:tt)*]
        Dword($a:literal), wlen: $w:literal => $handler:path $(, $($rest:tt)*)?
    ) => {
        $crate::router!(@munch [$cx, $content, $addr, $wlen, $ignore]
            [$($arms)* ($crate::Address::Dword($a), $w) =>
                $crate::router!(@call $cx, $content, $addr, $handler),]
            $($($rest)*)?)
    };
    (@munch [$cx:ident, $content:ident, $addr:ident, $wlen:ident, $ignore:literal] [$($arms:tt)*]
        Register($p:literal, $a:literal), wlen: $w:literal => $handler:path $(, $($rest:tt)*)?
    ) => {
        $crate::router!(@munch [$cx, $content, $addr, $wlen, $ignore]
            [$($arms)* ($crate::Address::Register($p, $a), $w) =>
                $crate::router!(@call $cx, $content, $addr, $handler),]
            $($($rest)*)?)
    };

    // No entries left: emit the match.
    (@munch [$cx:ident, $content:ident, $addr:ident, $wlen:ident, $ignore:literal] [$($arms:tt)*] ) => {
        match ($addr, $wlen) {
            $($arms)*
            _ => {
                if !$ignore {
                    return ::core::result::Result::Err($crate::Error::Unhandled($addr, $wlen));
                }
            }
        }
    };

    // Shared arm body: deserialize the payload, then hand it to its handler.
    // The payload type is inferred from the handler's argument.
    (@call $cx:ident, $content:ident, $addr:ident, $handler:path) => {{
        let val = $content
            .take()
            .map_err(|e| $crate::Error::Deserialize($addr, e))?;
        $handler($cx, val);
    }};
}
