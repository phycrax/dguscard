pub trait Serialize {
    const HEADER: u16;
    const CRC: bool;
    fn serialize(&self, buffer: &mut [u8]) -> &mut [u8];
}

pub struct DwinData<T> {
    addr: u16,
    val: T,
}

impl Serialize for DwinData<u16> {
    fn serialize(&self, buffer: &mut [u8]) -> &mut [u8] {
        assert!(buffer.len() >= 8, "Buffer too small");
        assert!(buffer.len() < u8::MAX as usize, "Buffer too large");
        buffer[0] = HEADER.to_be_bytes()[0];
        buffer[1] = HEADER.to_be_bytes()[1];
    }
}

pub struct Serializer {
    config: Config,
    data: &mut [u8],
    index: usize,
}

//pub fn to_dwin<T>(buffer: &mut [u8], )

impl Serializer {
    pub fn new(buffer: &mut [u8], config: Config) -> Self {
        assert!(buffer.len() >= 8, "Buffer too small");
        assert!(buffer.len() < u8::MAX as usize, "Buffer too large");
        let header = config.header;
        let mut frame = Self {
            config,
            data: buffer,
            index: 0,
        };
        frame.append(header); // -> [HEADER:2]
        frame.append(0u8); // -> [LEN:1]
        frame.append(command as u8); // -> [CMD:1]
        frame.append(address); // -> [ADDR:2]
        frame
    }
}
