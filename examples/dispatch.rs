extern crate dwin;

use dwin::{
    de::metadata_from_bytes, define_dispatcher, dispatcher::Dispatch, Config, DwinVariable,
};
use serde::Deserialize;

#[derive(Deserialize, Debug)]
struct Button {
    val: u16,
}

impl DwinVariable for Button {
    const ADDRESS: u16 = 0x1234;
}

impl Dispatch<'_> for Button {
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

impl Dispatch<'_> for Energy {
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

fn main() {
    let mut buf = [0u8; 20];
    let cfg = Config {
        header: 0x5AA5,
        crc: None,
    };

    define_dispatcher!(dwin_dispatch => Button, Energy);

    {
        read_button(&mut buf);
        dwin_dispatch(&buf, cfg.clone()).unwrap();
    }
    {
        read_energy(&mut buf);
        dwin_dispatch(&buf, cfg.clone()).unwrap();
    }
    // Should panic with DispatcherMetadataMismatch
    {
        read_unknown(&mut buf);
        dwin_dispatch(&buf, cfg.clone()).unwrap();
    }
}
