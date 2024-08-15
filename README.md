# Protocol
[HDR:   2]: Header frame
[LEN:   1]: Payload length (CMD..=CRC)
[CMD:   1]: DGUS Command, refer to DGUS DevGuide
[ADDR:  2]: Address of the DWIN variable
[WLEN:  1]: Word length
[DATA:  N]: Max 246(?) bytes. Each DWIN address holds 2 bytes, big endian per word
[CRC:   2]: can be disabled, uses CRC_16_MODBUS, little endian

Write commands return ACK.
ACK: [HDR:2][LEN:1][CMD:1]['O''K':2][CRC:2]