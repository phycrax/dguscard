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

use crc::{Crc, CRC_16_MODBUS};
pub use error::{Error, Result};

const CRC: Crc<u16> = Crc::<u16>::new(&CRC_16_MODBUS);
const HEADER: u16 = 0x5AA5;
