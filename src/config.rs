#[derive(Clone)]
pub struct Config {
    pub header1: u8,
    pub header2: u8,
    pub crc_enabled: bool,
}

impl Config {
    pub const fn new(header1: u8, header2: u8, crc_enabled: bool) -> Self {
        Self {
            header1,
            header2,
            crc_enabled,
        }
    }
}
