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

// pub struct Config<T> {
//     pub header: u16,
//     pub crc: bool,
//     pub crc_engine: T,
// }

// #[cfg(feature = "crc")]
// pub use crc::CrcEngine;

// #[cfg(feature = "crc")]
// impl Default for Config<CrcEngine> {
//     fn default() -> Self {
//         Self {
//             header: 0x5AA5,
//             crc: true,
//             crc_engine: CrcEngine,
//         }
//     }
// }

// #[repr(u8)]
// #[derive(PartialEq, Debug, Clone, Copy)]
// #[cfg_attr(feature = "defmt", derive(defmt::Format))]
// pub enum FrameCommand {
//     WriteRegister = 0x80,
//     ReadRegister,
//     Write16,
//     Read16,
//     WriteCurve,
//     Undefined,
//     Write32,
//     Read32,
// }

// impl From<u8> for FrameCommand {
//     fn from(value: u8) -> Self {
//         use FrameCommand::*;
//         match value {
//             0x80 => WriteRegister,
//             0x81 => ReadRegister,
//             0x82 => Write16,
//             0x83 => Read16,
//             0x84 => WriteCurve,
//             0x86 => Write32,
//             0x87 => Read32,
//             _ => Undefined,
//         }
//     }
// }

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
