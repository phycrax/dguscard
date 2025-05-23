#![no_std]
#![warn(missing_docs)]
#![cfg_attr(not(doctest), doc = include_str!("../README.md"))]
#![cfg_attr(docsrs, feature(doc_cfg))]

//! ## Feature flags
#![doc = document_features::document_features!(feature_label = r#"<span class="stab portability"><code>{feature}</code></span>"#)]

mod error;
pub mod instruction;
pub mod request;
pub mod response;

#[cfg(feature = "experimental")]
/// Experimental features
pub mod dispatch;

use crc::{Crc, CRC_16_MODBUS};
pub use error::{Error, Result};

const CRC: Crc<u16> = Crc::<u16>::new(&CRC_16_MODBUS);
const HEADER: u16 = 0x5AA5;

use instruction::Instruction;
use request::Request;
use serde::Serialize;

/// ToRequest trait
/// 
/// Users can implement this trait on their types to be able to use to_x functions
pub trait ToRequest<T: Instruction> {
    /// Command to be assigned to the type
    const CMD: T;
}

/// Serialize a `T` to the given slice, with the resulting slice containing
/// data in a serialized format.
pub fn to_slice<'b, I: Instruction, T: ToRequest<I> + Serialize>(
    val: &T,
    buf: &'b mut [u8],
    crc: bool,
) -> Result<&'b mut [u8]> {
    let mut request = Request::with_slice(buf, T::CMD)?;
    request.push(val)?;
    request.finalize(crc)
}

#[cfg(test)]
mod tests {
    use crate::{
        instruction::{Word, Write},
        to_slice, ToRequest,
    };
    use serde::Serialize;

    #[derive(Serialize)]
    struct Test(u16);
    impl ToRequest<Word<Write>> for Test {
        const CMD: Word<Write> = Word {
            addr: 0x1234,
            cmd: Write,
        };
    }

    #[test]
    pub fn slice() {
        let mut buf = [0u8; 10];
        let _ = to_slice(&Test(0), &mut buf, true).unwrap();
    }
}
