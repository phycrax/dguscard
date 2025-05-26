use crate::response::ResponseData;

/// TODO
pub trait Dispatch<Key> {
    #[allow(async_fn_in_trait)]
    /// TODO
    async fn handle(&mut self, key: &Key, data: ResponseData);
}

#[macro_export]
/// Define a dispatch struct with a context and a handler for each endpoint.
macro_rules! define_dispatch {
    (name: $name:ident; key: $key_ty:ty; context: $context_ty:ty; endpoints: {$($key:tt => $handler:ident)+};) => {
        pub struct $name {
            pub context: $context_ty,
        }

        impl $name {
            pub fn new(context: $context_ty) -> Self {
                Self { context }
            }
        }

        impl Dispatch<$key_ty> for $name {
            async fn handle(&mut self, key: &$key_ty, data: dguscard::response::ResponseData<'_>) {
                match key {
                    $(
                        $key => $handler(&mut self.context, data).await,
                    )+
                    _ => defmt::error!("Unknown response"),
                }
            }
        }
    }
}

/// Serialize a `T` as a request with the command to the given slice,
/// with the resulting slice containing data in a serialized format.
///
/// # Example
///
/// ```rust
/// use dguscard::{request::to_slice, command::{Word, Write}};
/// # use std::io::Write as IoWrite;
/// #[derive(serde::Serialize)]
/// struct MyData {
///     byte_h: u8,
///     byte_l: u8,
///     word: u16,
///     dword: u32,
///     float: f32,
///     double: f64,
/// }
/// let data = MyData { byte_h: 0, byte_l: 1, word: 2, dword: 3, float: 4.0, double: 5.0 };
///
/// let mut uart =
/// # Vec::new();
/// // Backing buffer for the request.
/// let buf = &mut [0u8; 50];
/// // Serialize data to a slice buffer/output type with write word command and crc.
/// let mut frame = to_slice(&data, buf, Word { addr: 0x1234, cmd: Write}, true).unwrap();
/// // Transmit the frame
/// uart.write_all(frame).unwrap();
/// ```
///
pub fn to_slice<'b, T: Serialize, C: Command>(
    val: &T,
    buf: &'b mut [u8],
    cmd: C,
    crc: bool,
) -> Result<&'b mut [u8]> {
    let mut request = Request::with_slice(buf, cmd)?;
    request.push(val)?;
    request.finalize(crc)
}

/// Serialize a `T` as a request with the command to a [`Vec<u8, N>`][heapless::Vec],
/// with the `Vec` containing data in a serialized format.
///
/// # Example
///
/// ```rust
/// use dguscard::{request::to_hvec, command::{Word, Write}};
/// # use std::io::Write as IoWrite;
/// #[derive(serde::Serialize)]
/// struct MyData {
///     byte_h: u8,
///     byte_l: u8,
///     word: u16,
///     dword: u32,
///     float: f32,
///     double: f64,
/// }
/// let data = MyData { byte_h: 0, byte_l: 1, word: 2, dword: 3, float: 4.0, double: 5.0 };
///
/// let mut uart =
/// # Vec::new();
/// // Serialize data as a [`Vec<u8, N>`][heapless::Vec] with write word command and crc.
/// let mut frame: heapless::Vec<u8, 32> = to_hvec(&data, Word { addr: 0x1234, cmd: Write}, true).unwrap();
/// // Transmit the frame
/// uart.write_all(&frame).unwrap();
/// ```
///
#[cfg(feature = "heapless")]
pub fn to_hvec<T: Serialize, C: Command, const N: usize>(
    val: &T,
    cmd: C,
    crc: bool,
) -> Result<heapless::Vec<u8, N>> {
    let mut request = Request::with_hvec(cmd)?;
    request.push(val)?;
    request.finalize(crc)
}

#[cfg(test)]
mod tests {
    use crate::{
        command::{Read, Word, Write},
        request::{to_hvec, to_slice},
        response::Response,
    };
    use serde::Serialize;

    #[derive(Serialize)]
    struct Test(u16);

    #[test]
    pub fn request_slice() {
        let mut buf = [0u8; 10];
        let _ = to_slice(
            &Test(0),
            &mut buf,
            Word {
                addr: 0x1234,
                cmd: Write,
            },
            true,
        )
        .unwrap();
    }

    #[test]
    pub fn request_hvec() {
        let _: heapless::Vec<u8, 10> = to_hvec(
            &Test(0),
            Word {
                addr: 0x1234,
                cmd: Write,
            },
            true,
        )
        .unwrap();
    }

    #[test]
    fn response_word_data() {
        let input = [
            0x5A, 0xA5, 8, 0x83, 0x12, 0x34, 2, b'D', b'G', b'U', b'S', 1, 2, 3, 4,
        ];
        let (response, rest) = Response::take_from_bytes(&input, false).unwrap();
        let Response::WordData { cmd, mut content } = response else {
            panic!("Unexpected response type");
        };
        let content: [u8; 4] = content.take().unwrap();
        assert_eq!(
            cmd,
            Word {
                addr: 0x1234,
                cmd: Read { wlen: 2 }
            }
        );
        assert_eq!(content, [b'D', b'G', b'U', b'S',]);
        assert_eq!(rest, &[1, 2, 3, 4]);
    }
}
