# Protocol
[HDR:2][LEN:1][CMD:1][ADDR:2][WLEN:1][DATA:N][CRC:2]
HDR: Header frames
LEN: Size of the packet starting from CMD, includes CRC
CMD: Refer to DGUS DevGuide
ADDR: Address of the DWIN variable
CRC: is optional, uses CRC_16_MODBUS, little endian
DATA: Max 246 bytes. Each DWIN address holds 2 bytes, big endian
WLEN: byte, word or dword length based on command
Exceptions: Write commands return ACK.
ACK: [HDR:2][LEN:1][CMD:1]['O''K':2][CRC:2]