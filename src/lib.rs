#![no_std]

pub mod de;
pub mod error;
pub mod ser;

use crc::{Crc, CRC_16_MODBUS};
const CRC: crc::Crc<u16> = Crc::<u16>::new(&CRC_16_MODBUS);

#[derive(Copy, Clone)]
pub struct Config {
    pub header: u16,
    pub crc: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            header: 0x5AA5,
            crc: true,
        }
    }
}

pub trait DwinVariable {
    const ADDRESS: u16;
}

pub fn to_request<T: DwinVariable + Sized>(buf: &mut [u8], cfg: Config) -> error::Result<&[u8]> {
    let mut serializer =
        ser::serializer::Serializer::new(buf, cfg.header, Command::Read, T::ADDRESS)?;
    serializer.push_byte((core::mem::size_of::<T>() / 2) as u8)?;
    serializer.finalize(cfg.crc)
}

#[repr(u8)]
#[derive(PartialEq, Debug, Clone, Copy)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Command {
    Write = 0x82,
    Read,
    // ToDo other cmds
    Undefined,
}

impl From<u8> for Command {
    fn from(value: u8) -> Self {
        use Command::*;
        match value {
            0x82 => Write,
            0x83 => Read,
            _ => Undefined,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let output = to_request::<Energy>(&mut buf, Default::default()).unwrap();
        assert_eq!(output, expected);
    }
}

//device commands
/*
void DWIN_ReadVP(uint16_t vAdd, uint8_t vSize) {
  DWIN_AddByte(DWIN_CMD_VAR_R);
  DWIN_AddWord(vAdd);
  DWIN_AddByte(vSize);
  DWIN_SendPack();
}

void DWIN_SetPage(uint16_t pAdd) {
  DWIN_AddByte(DWIN_CMD_VAR_W);
  DWIN_AddWord(DWIN_VADD_PIC_SET);
  DWIN_AddWord(0x5A01);
  DWIN_AddWord(pAdd);
  DWIN_SendPack();
}

void DWIN_SetBrightness(uint8_t level, uint16_t time) {
  DWIN_AddByte(DWIN_CMD_VAR_W);
  DWIN_AddWord(DWIN_VADD_LED_CFG);
  DWIN_AddByte(level);
  DWIN_AddByte(level / 2);
  DWIN_AddWord(time);
  DWIN_SendPack();

  DWIN_AddByte(DWIN_CMD_VAR_W);
  DWIN_AddWord(0x0512);
  DWIN_AddByte(0x5A);
  DWIN_AddByte(level);
  DWIN_SendPack();
}

void DWIN_SetBackgroundIcl(uint16_t icl) {
  DWIN_AddByte(DWIN_CMD_VAR_W);
  DWIN_AddWord(DWIN_VADD_ICL_SET);
  DWIN_AddWord(0x5A00);
  DWIN_AddWord(icl);
  DWIN_SendPack();
}


*/
