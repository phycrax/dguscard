use crate::{
    error::{Error, Result},
    Config, DwinVariable,
};
use serde::Serialize;
use serializer::Serializer;

pub(crate) mod serializer;

pub fn to_slice<'b, T>(value: &T, buf: &'b mut [u8], config: Config) -> Result<&'b [u8]>
where
    T: Serialize + DwinVariable,
{
    let mut serializer = Serializer::new(buf, config.header, T::ADDRESS);
    value.serialize(&mut serializer)?;
    serializer.finalize(config.crc)
}

#[cfg(test)]
mod tests {
    use super::*;
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
    fn buffer_short() {
        let mut buf = [0u8; 10];
        let bg = BackgroundIcl::new(0x1234); //needs 12 with crc
        match to_slice(&bg, &mut buf, Default::default()) {
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
        let output = to_slice(&bg, &mut buf, Default::default()).unwrap();

        assert_eq!(output, &expected);
    }

    #[test]
    fn set_background_icl_nocrc() {
        let expected = [0x5Au8, 0xA5, 7, 0x82, 0x00, 0xDE, 0x5A, 0x00, 0x12, 0x34];

        let mut buf = [0u8; 50];
        let bg = BackgroundIcl::new(0x1234);
        let output = to_slice(
            &bg,
            &mut buf,
            Config {
                header: 0x5AA5,
                crc: false,
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
        let output = to_slice(
            &bg,
            &mut buf,
            Config {
                header: 0xB44B,
                crc: false,
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
        match to_slice(&not_yet, &mut buf, Default::default()) {
            Err(Error::NotYetImplemented) => (),
            _ => panic!("u8 impl not ready"),
        }
    }
}
