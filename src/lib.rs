#![no_std]

pub mod config;
pub mod packet;
pub mod parser;
pub mod widget;

use crate::config::Config;
use arrayvec::ArrayVec;
use crcxx::crc16::{catalog::CRC_16_MODBUS, *};

const CRC: Crc<LookupTable256> = Crc::<LookupTable256>::new(&CRC_16_MODBUS);
const MAX_DATA: usize = 64;
const MAX_WIDGET: usize = 10;
const PACKET_MAX_SIZE: usize = 64;

#[repr(u8)]
pub enum Cmd {
    WriteRegister = 0x80,
    ReadRegister,
    Write16,
    Read16,
    WriteCurve,
    Undefined,
    Write32,
    Read32,
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
