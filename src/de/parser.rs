use crate::{
    error::{Error, Result},
    Command, Config,
};

/// Metadata
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Metadata {
    pub cmd: Command,
    pub addr: u16,
    pub wlen: u8,
}

pub struct FrameBytes<'a>(pub &'a [u8]);
pub struct DataBytes<'a>(pub &'a [u8]);

/// Splits frame from a byte slice and returns the frame bytes within the byte slice and the rest of the slice.
pub fn split_frame(input: &[u8], cfg: Config) -> Result<(FrameBytes, &[u8])> {
    // Strip header from input
    let input = input
        .strip_prefix(&u16::to_be_bytes(cfg.header))
        .ok_or(Error::DeserializeBadHeader)?;

    // Strip length from input
    let (len, input) = input.split_first().ok_or(Error::DeserializeUnexpectedEnd)?;
    let len = *len as usize;

    // Split input with the length
    let (input, rest) = input
        .split_at_checked(len)
        .ok_or(Error::DeserializeUnexpectedEnd)?;

    // Strip CRC from input
    let input = if let Some(mut digest) = cfg.crc {
        let (input, crc) = input
            .split_last_chunk()
            .ok_or(Error::DeserializeUnexpectedEnd)?;
        digest.update(input);
        if u16::from_le_bytes(*crc) != digest.finalize() {
            return Err(Error::DeserializeBadCrc);
        }
        input
    } else {
        input
    };

    // Return the trimmed data and the rest
    Ok(({ FrameBytes(input) }, rest))
}

/// Splits metadata from a frame slice and returns the metadata and the data bytes.
pub fn split_metadata(FrameBytes(input): FrameBytes) -> Result<(Metadata, DataBytes)> {
    // Strip command from input
    let (cmd, input) = input.split_first().unwrap();
    let cmd = Command::from(*cmd);
    if cmd == Command::Undefined {
        return Err(Error::DeserializeBadCommand);
    }

    // Strip address from input
    let (addr, input) = input.split_first_chunk().unwrap();
    let addr = u16::from_be_bytes(*addr);

    // Strip word length from input, if there is none (could be ACK), set to 0
    let (wlen, input) = input.split_first().unwrap_or((&0, input));
    let wlen = *wlen;

    // Calculate the actual raw
    Ok((Metadata { cmd, addr, wlen }, DataBytes(input)))
}

#[cfg(test)]
mod tests {
    use super::*;
}
