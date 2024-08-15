pub(crate) mod output;
pub(crate) mod serializer;

use crate::{
    error::Result,
    ser::{
        output::{Output, Slice},
        serializer::Serializer,
    },
    Command, Config,
};
use serde::Serialize;

#[cfg(feature = "heapless")]
use heapless::Vec;

pub struct DataSerializer<'a, T> {
    addr: u16,
    cfg: Config,
    val: &'a T,
}

impl<'a, 'b, T> DataSerializer<'a, T> {
    pub fn new(val: &'a T, addr: u16, cfg: Config) -> Self {
        Self { val, addr, cfg }
    }

    pub fn to_slice(&self, buf: &'b mut [u8]) -> Result<&'b mut [u8]>
    where
        T: Serialize,
    {
        let mut serializer = Serializer::new(Slice::new(buf), self.cfg, Command::Write, self.addr)?;
        self.val.serialize(&mut serializer)?;
        serializer.finalize()
    }

    #[cfg(feature = "heapless")]
    pub fn to_hvec<const N: usize>(&self) -> Result<Vec<u8, N>>
    where
        T: Serialize,
    {
        let mut serializer = Serializer::new(Vec::new(), self.cfg, Command::Write, self.addr)?;
        self.val.serialize(&mut serializer)?;
        serializer.finalize()
    }
}

pub struct RequestSerializer {
    addr: u16,
    wlen: u8,
    cfg: Config,
}

impl RequestSerializer {
    pub fn with_type<T: Sized>(addr: u16, cfg: Config) -> Self {
        const {
            use core::mem::size_of;
            assert!(size_of::<T>() % 2 == 0, "Type size must be even");
            assert!(
                size_of::<T>() <= u8::MAX as usize,
                "Type must be smaller than u8::MAX"
            );
        }
        Self {
            addr,
            wlen: core::mem::size_of::<T>() as u8 / 2,
            cfg,
        }
    }

    pub fn with_wlen(wlen: u8, addr: u16, cfg: Config) -> Self {
        Self { addr, wlen, cfg }
    }

    pub fn to_slice<'b>(&self, buf: &'b mut [u8]) -> Result<&'b mut [u8]> {
        let mut serializer = Serializer::new(Slice::new(buf), self.cfg, Command::Read, self.addr)?;
        serializer.output.try_push(self.wlen)?;
        serializer.finalize()
    }

    #[cfg(feature = "heapless")]
    pub fn to_hvec<const N: usize>(&self) -> Result<Vec<u8, N>> {
        let mut serializer = Serializer::new(Vec::new(), self.cfg, Command::Read, self.addr)?;
        serializer.output.try_push(self.wlen)?;
        serializer.finalize()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::{Error, Result};

    #[derive(Serialize)]
    struct TestTuple(u16, u16);

    impl TestTuple {
        pub fn new(id: u16) -> Self {
            Self(0x5A00, id)
        }
    }

    #[test]
    fn buffer_short_slice() {
        let mut buf = [0u8; 10];
        let data = TestTuple::new(0x1234); //needs 12 with crc
        let send = DataSerializer::new(&data, 0x00DE, Default::default());
        match send.to_slice(&mut buf) {
            Err(Error::SerializeBufferFull) => (),
            _ => panic!("should return buffer full"),
        }
    }

    #[test]
    fn buffer_short_vec() {
        let data = TestTuple::new(0x1234);
        let send = DataSerializer::new(&data, 0x00DE, Default::default());
        let result: Result<Vec<u8, 10>> = send.to_hvec(); //needs 12 with crc
        match result {
            Err(Error::SerializeBufferFull) => (),
            _ => panic!("should return buffer full"),
        }
    }

    #[test]
    fn set_background_icl_output_crc() {
        let expected = [
            0x5Au8, 0xA5, 9, 0x82, 0x00, 0xDE, 0x5A, 0x00, 0x12, 0x34, 0x0e, 0xb4,
        ];

        let mut buf = [0u8; 50];
        let data = TestTuple::new(0x1234);
        let send = DataSerializer::new(&data, 0x00DE, Default::default());
        let output = send.to_slice(&mut buf).unwrap();

        assert_eq!(output, &expected);
    }

    #[test]
    fn set_background_icl_output_crc_hvec() {
        let expected = [
            0x5Au8, 0xA5, 9, 0x82, 0x00, 0xDE, 0x5A, 0x00, 0x12, 0x34, 0x0e, 0xb4,
        ];
        let expected: Vec<u8, 12> = Vec::from_slice(&expected).unwrap();
        let data = TestTuple::new(0x1234);
        let send = DataSerializer::new(&data, 0x00DE, Default::default());
        let output: Vec<u8, 12> = send.to_hvec().unwrap();

        assert_eq!(output, expected);
    }

    #[test]
    fn set_background_icl_nocrc() {
        let expected = [0x5Au8, 0xA5, 7, 0x82, 0x00, 0xDE, 0x5A, 0x00, 0x12, 0x34];
        let mut buf = [0u8; 50];
        let data = TestTuple::new(0x1234);
        let send = DataSerializer::new(
            &data,
            0x00DE,
            Config {
                header: 0x5AA5,
                crc: None,
            },
        );
        let output = send.to_slice(&mut buf).unwrap();
        assert_eq!(output, &expected);
    }

    #[test]
    fn set_background_icl_nocrc_header() {
        let expected = [0xB4u8, 0x4B, 7, 0x82, 0x00, 0xDE, 0x5A, 0x00, 0x12, 0x34];
        let mut buf = [0u8; 50];
        let data = TestTuple::new(0x1234);
        let send = DataSerializer::new(
            &data,
            0x00DE,
            Config {
                header: 0xB44B,
                crc: None,
            },
        );
        let output = send.to_slice(&mut buf).unwrap();
        assert_eq!(output, &expected);
    }

    #[test]
    fn not_yet_u8_tuple() {
        #[derive(Serialize)]
        struct NotYetImpl(u8);

        let mut buf = [0u8; 50];
        let not_yet = NotYetImpl(123);
        let send = DataSerializer::new(&not_yet, 0x00DE, Default::default());
        match send.to_slice(&mut buf) {
            Err(Error::NotYetImplemented) => (),
            _ => panic!("u8 impl not ready"),
        }
    }

    #[test]
    fn request() {
        struct TestReq {
            _u: u16,
        }
        let expected = [0x5Au8, 0xA5, 6, 0x83, 0x00, 0x0F, 1, 0xED, 0x90];
        let mut buf = [0u8; 9];
        let request = RequestSerializer::with_type::<TestReq>(0x000F, Default::default());
        let output = request.to_slice(&mut buf).unwrap();
        assert_eq!(output, expected);
    }
}
