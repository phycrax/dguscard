#![no_std]
#![warn(missing_docs)]
#![cfg_attr(not(doctest), doc = include_str!("../README.md"))]
#![cfg_attr(docsrs, feature(doc_cfg))]

//! ## Feature flags
#![doc = document_features::document_features!(feature_label = r#"<span class="stab portability"><code>{feature}</code></span>"#)]

mod error;

pub mod command;
pub mod request;
pub mod response;

#[cfg(feature = "experimental")]
/// Experimental features
pub mod dispatch;

use crc::{Crc, CRC_16_MODBUS};
pub use error::{Error, Result};
pub use request::to_slice;

#[cfg(feature = "heapless")]
pub use request::to_hvec;

const CRC: Crc<u16> = Crc::<u16>::new(&CRC_16_MODBUS);
const HEADER: u16 = 0x5AA5;

#[cfg(test)]
mod tests {
    use crate::{
        command::{Word, Write},
        to_hvec, to_slice,
    };
    use serde::Serialize;

    #[derive(Serialize)]
    struct Test(u16);

    #[test]
    pub fn slice() {
        let mut buf = [0u8; 10];
        let _ = to_slice(
            &Test(0),
            &mut buf,
            Word {
                addr: 0x1234,
                cmd: Write,
            },
            true,
        )
        .unwrap();
    }

    #[test]
    pub fn hvec() {
        let _: heapless::Vec<u8, 10> = to_hvec(
            &Test(0),
            Word {
                addr: 0x1234,
                cmd: Write,
            },
            true,
        )
        .unwrap();
    }
}
