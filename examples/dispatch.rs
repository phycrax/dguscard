extern crate dwin;

use dwin::{
    de::{from_raw_bytes, metadata_from_bytes},
    Config, DwinVariable,
};
use serde::Deserialize;

trait DwinDispatch<'a>: DwinVariable + Deserialize<'a> {
    fn handler(&self);
}

macro_rules! define_dispatch {
    ($fn_name:ident => $($type:ident),+) => {
        fn $fn_name(md: dwin::MetaData, rb: dwin::de::RawBytes) -> dwin::error::Result<()> {
            $(
            if md == $type::metadata() {
                let val: $type = from_raw_bytes(rb)?;
                val.handler();
                return Ok(());
            })+
            Err(dwin::error::Error::Dispatch)
        }
    }
}

#[derive(Deserialize, Debug)]
struct Button {
    val: u16,
}

impl DwinVariable for Button {
    const ADDRESS: u16 = 0x1234;
}

impl DwinDispatch<'_> for Button {
    fn handler(&self) {
        print!("button: {}", self.val);
    }
}

#[derive(Deserialize, Debug)]
struct Energy {
    val: u16,
}

impl DwinVariable for Energy {
    const ADDRESS: u16 = 0x1235;
}

impl DwinDispatch<'_> for Energy {
    fn handler(&self) {
        println!("energy: {}", self.val);
    }
}

fn read_button(buf: &mut [u8]) {
    let received = [
        0x5A, 0xA5, 6, 0x83, 0x12, 0x34, 1, 0xCC, 0xDD, 0, 0, 0, 0, 0, 0,
    ];
    buf[..received.len()].clone_from_slice(&received);
}

fn read_energy(buf: &mut [u8]) {
    let received = [
        0x5A, 0xA5, 6, 0x83, 0x12, 0x35, 1, 0xEE, 0xFF, 0, 0, 0, 0, 0, 0,
    ];
    buf[..received.len()].clone_from_slice(&received);
}

fn read_unknown(buf: &mut [u8]) {
    let received = [
        0x5A, 0xA5, 6, 0x83, 0x12, 0x36, 1, 0xEE, 0xFF, 0, 0, 0, 0, 0, 0,
    ];
    buf[..received.len()].clone_from_slice(&received);
}

define_dispatch!(dwin_dispatch => Energy, Button);

fn main() {
    let mut buf = [0u8; 20];
    let cfg = Config {
        header: 0x5AA5,
        crc: None,
    };
    {
        read_button(&mut buf);
        let (md, rb) = metadata_from_bytes(&buf, cfg.clone()).unwrap();
        dwin_dispatch(md, rb).unwrap();
    }
    {
        read_energy(&mut buf);
        let (md, rb) = metadata_from_bytes(&buf, cfg.clone()).unwrap();
        dwin_dispatch(md, rb).unwrap();
    }
    {
        read_unknown(&mut buf);
        let (md, rb) = metadata_from_bytes(&buf, cfg.clone()).unwrap();
        dwin_dispatch(md, rb).unwrap();
    }
}
