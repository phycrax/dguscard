pub(crate) mod output;
pub(crate) mod serializer;

use crate::{
    error::Result,
    ser::{
        output::{Output, Slice},
        serializer::Serializer,
    },
    Command, Config, DwinVariable,
};
use serde::Serialize;

pub fn send_to_slice<'b, T>(value: &T, buf: &'b mut [u8], cfg: Config) -> Result<&'b mut [u8]>
where
    T: DwinVariable + Serialize,
{
    let mut serializer = Serializer {
        output: Slice::new(buf),
    };
    serializer.init(cfg.header, Command::Write, T::ADDRESS)?;
    value.serialize(&mut serializer)?;
    serializer.finalize(cfg.crc)
}

pub fn request_to_slice<'b, T>(buf: &'b mut [u8], cfg: Config) -> Result<&'b mut [u8]>
where
    T: DwinVariable,
{
    let metadata = T::metadata();
    let mut serializer = Serializer {
        output: Slice::new(buf),
    };
    serializer.init(cfg.header, Command::Read, metadata.addr)?;
    serializer.output.try_push(metadata.wlen)?;
    serializer.finalize(cfg.crc)
}

#[cfg(feature = "heapless")]
use heapless::Vec;

#[cfg(feature = "heapless")]
pub fn send_to_vec<T, const N: usize>(value: &T, cfg: Config) -> Result<Vec<u8, N>>
where
    T: DwinVariable + Serialize,
{
    // todo update assert
    const { assert!(N > 10) }
    let mut serializer = Serializer { output: Vec::new() };
    serializer.init(cfg.header, Command::Write, T::ADDRESS)?;
    value.serialize(&mut serializer)?;
    serializer.finalize(cfg.crc)
}

#[cfg(feature = "heapless")]
pub fn request_to_vec<T, const N: usize>(cfg: Config) -> Result<Vec<u8, N>>
where
    T: DwinVariable,
{
    // todo update assert
    const { assert!(N > 10) }
    let metadata = T::metadata();
    let mut serializer = Serializer { output: Vec::new() };
    serializer.init(cfg.header, Command::Read, T::ADDRESS)?;
    serializer.output.try_push(metadata.wlen)?;
    serializer.finalize(cfg.crc)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::{Error, Result};
    #[derive(Serialize)]
    struct BackgroundIcl(u16, u16);

    impl BackgroundIcl {
        pub fn new(id: u16) -> Self {
            Self(0x5A00, id)
        }
    }

    impl DwinVariable for BackgroundIcl {
        const ADDRESS: u16 = 0x00DE;
    }

    #[test]
    fn buffer_short_slice() {
        let mut buf = [0u8; 10];
        let bg = BackgroundIcl::new(0x1234); //needs 12 with crc
        match send_to_slice(&bg, &mut buf, Default::default()) {
            Err(Error::SerializeBufferFull) => (),
            _ => panic!("should return buffer full"),
        }
    }

    #[test]
    fn buffer_short_vec() {
        let bg = BackgroundIcl::new(0x1234);
        let result: Result<Vec<u8, 10>> = send_to_vec(&bg, Default::default()); //needs 12 with crc
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
        let bg = BackgroundIcl::new(0x1234);
        let output = send_to_slice(&bg, &mut buf, Default::default()).unwrap();

        assert_eq!(output, &expected);
    }

    #[test]
    fn set_background_icl_output_crc_hvec() {
        let expected = [
            0x5Au8, 0xA5, 9, 0x82, 0x00, 0xDE, 0x5A, 0x00, 0x12, 0x34, 0x0e, 0xb4,
        ];

        let expected: Vec<u8, 12> = heapless::Vec::from_slice(&expected).unwrap();
        let bg = BackgroundIcl::new(0x1234);
        let output: Vec<u8, 12> = send_to_vec(&bg, Default::default()).unwrap();

        assert_eq!(output, expected);
    }

    #[test]
    fn set_background_icl_nocrc() {
        let expected = [0x5Au8, 0xA5, 7, 0x82, 0x00, 0xDE, 0x5A, 0x00, 0x12, 0x34];

        let mut buf = [0u8; 50];
        let bg = BackgroundIcl::new(0x1234);
        let output = send_to_slice(
            &bg,
            &mut buf,
            Config {
                header: 0x5AA5,
                crc: None,
            },
        )
        .unwrap();

        assert_eq!(output, &expected);
    }

    #[test]
    fn set_background_icl_nocrc_header() {
        let expected = [0xB4u8, 0x4B, 7, 0x82, 0x00, 0xDE, 0x5A, 0x00, 0x12, 0x34];

        let mut buf = [0u8; 50];
        let bg = BackgroundIcl::new(0x1234);
        let output = send_to_slice(
            &bg,
            &mut buf,
            Config {
                header: 0xB44B,
                crc: None,
            },
        )
        .unwrap();

        assert_eq!(output, &expected);
    }

    #[derive(Serialize)]
    struct NotYetImpl(u8);

    impl DwinVariable for NotYetImpl {
        const ADDRESS: u16 = 0x3456;
    }

    #[test]
    fn not_yet_u8_tuple() {
        let mut buf = [0u8; 50];
        let not_yet = NotYetImpl(123);
        match send_to_slice(&not_yet, &mut buf, Default::default()) {
            Err(Error::NotYetImplemented) => (),
            _ => panic!("u8 impl not ready"),
        }
    }

    struct Energy {
        _u: u16,
    }

    impl DwinVariable for Energy {
        const ADDRESS: u16 = 0x000F;
    }

    #[test]
    fn request() {
        let expected = [0x5Au8, 0xA5, 6, 0x83, 0x00, 0x0F, 1, 0xED, 0x90];
        let mut buf = [0u8; 9];
        let output = request_to_slice::<Energy>(&mut buf, Default::default()).unwrap();
        assert_eq!(output, expected);
    }
}
