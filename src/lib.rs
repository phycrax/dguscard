#![no_std]

pub mod de;
pub mod error;
pub mod ser;

pub trait DwinVariable {
    const ADDRESS: u16;
}

#[derive(Clone)]
pub struct Config<'a> {
    pub header: u16,
    pub crc: Option<crc::Digest<'a, u16>>,
}

impl<'a> Default for Config<'a> {
    fn default() -> Self {
        use crc::{Crc, CRC_16_MODBUS};
        const CRC: crc::Crc<u16> = Crc::<u16>::new(&CRC_16_MODBUS);
        Self {
            header: 0x5AA5,
            crc: Some(CRC.digest()),
        }
    }
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
