// ? Maybe get rid of trait and let users give their own fns in define_dispatch macro?
// ? The macro does not use this trait for bounds, use internal generic fn with this trait?
pub trait Dispatch {
    fn handler(&self);
}

#[macro_export]
macro_rules! define_dispatcher {
    ($fn_name:ident => $($type:ident),+) => {
        fn $fn_name(buf: &[u8], cfg: $crate::Config) -> $crate::error::Result<()> {
            let (md, rb) = $crate::de::metadata_from_bytes(&buf, cfg.clone())?;
            $(
            if md == $type::metadata() {
                let val: $type = $crate::de::from_raw_bytes(rb)?;
                val.handler();
                return Ok(());
            })+
            Err($crate::error::Error::DispatchMetadataMismatch)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Config, Variable};
    use serde::Deserialize;

    #[derive(Deserialize, Debug)]
    struct Button {
        val: u16,
    }

    impl Variable for Button {
        const ADDRESS: u16 = 0x1234;
    }

    impl Dispatch for Button {
        fn handler(&self) {
            assert_eq!(0xCCDD, self.val);
        }
    }

    define_dispatcher!(dwin_dispatch => Button);

    #[test]
    fn dispatch_button() {
        let received = [
            0x5A, 0xA5, 6, 0x83, 0x12, 0x34, 1, 0xCC, 0xDD, 0, 0, 0, 0, 0, 0,
        ];
        let cfg = Config {
            header: 0x5AA5,
            crc: None,
        };
        dwin_dispatch(&received, cfg).unwrap();
    }
}
