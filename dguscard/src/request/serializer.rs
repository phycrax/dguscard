use serde::{Serialize, ser};

use super::{Error, Storage};

/// `serde` compatible serializer.
///
/// Serialization output type is generic and must implement the [`Storage`] trait.
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Serializer<S: Storage> {
    /// This is the Storage that will be used to store any bytes generated
    /// by serialization
    pub output: S,
}

impl<S: Storage> ser::Serializer for &'_ mut Serializer<S> {
    type Ok = ();

    type Error = Error;

    type SerializeSeq = Self;
    type SerializeTuple = Self;
    type SerializeTupleStruct = Self;
    type SerializeTupleVariant = Self;
    type SerializeMap = Self;
    type SerializeStruct = Self;
    type SerializeStructVariant = Self;

    #[inline]
    fn is_human_readable(&self) -> bool {
        false
    }

    #[inline]
    fn serialize_bool(self, v: bool) -> Result<(), Error> {
        self.serialize_u16(u16::from(v))
    }

    #[inline]
    fn serialize_i8(self, v: i8) -> Result<(), Error> {
        self.output.try_extend(&v.to_be_bytes())
    }

    #[inline]
    fn serialize_i16(self, v: i16) -> Result<(), Error> {
        self.output.try_extend(&v.to_be_bytes())
    }

    #[inline]
    fn serialize_i32(self, v: i32) -> Result<(), Error> {
        self.output.try_extend(&v.to_be_bytes())
    }

    #[inline]
    fn serialize_i64(self, v: i64) -> Result<(), Error> {
        self.output.try_extend(&v.to_be_bytes())
    }

    #[inline]
    fn serialize_i128(self, v: i128) -> Result<(), Error> {
        self.output.try_extend(&v.to_be_bytes())
    }

    #[inline]
    fn serialize_u8(self, v: u8) -> Result<(), Error> {
        self.output.try_extend(&v.to_be_bytes())
    }

    #[inline]
    fn serialize_u16(self, v: u16) -> Result<(), Error> {
        self.output.try_extend(&v.to_be_bytes())
    }

    #[inline]
    fn serialize_u32(self, v: u32) -> Result<(), Error> {
        self.output.try_extend(&v.to_be_bytes())
    }

    #[inline]
    fn serialize_u64(self, v: u64) -> Result<(), Error> {
        self.output.try_extend(&v.to_be_bytes())
    }

    #[inline]
    fn serialize_u128(self, v: u128) -> Result<(), Error> {
        self.output.try_extend(&v.to_be_bytes())
    }

    #[inline]
    fn serialize_f32(self, v: f32) -> Result<(), Error> {
        self.output.try_extend(&v.to_be_bytes())
    }

    #[inline]
    fn serialize_f64(self, v: f64) -> Result<(), Error> {
        self.output.try_extend(&v.to_be_bytes())
    }

    #[inline]
    fn serialize_char(self, v: char) -> Result<(), Error> {
        let mut buf = [0u8; 4];
        self.output.try_extend(v.encode_utf8(&mut buf).as_bytes())
    }

    #[inline]
    fn serialize_str(self, v: &str) -> Result<(), Error> {
        self.output.try_extend(v.as_bytes())
    }

    #[inline]
    fn serialize_bytes(self, v: &[u8]) -> Result<(), Error> {
        self.output.try_extend(v)
    }

    #[inline]
    fn serialize_none(self) -> Result<(), Error> {
        Err(Error::Unsupported)
    }

    #[inline]
    fn serialize_some<T>(self, _v: &T) -> Result<(), Error>
    where
        T: ?Sized + Serialize,
    {
        Err(Error::Unsupported)
    }

    #[inline]
    fn serialize_unit(self) -> Result<(), Error> {
        Ok(())
    }

    #[inline]
    fn serialize_unit_struct(self, _name: &'static str) -> Result<(), Error> {
        Ok(())
    }

    #[inline]
    fn serialize_unit_variant(
        self,
        _name: &'static str,
        variant_index: u32,
        _variant: &'static str,
    ) -> Result<(), Error> {
        self.serialize_u16(variant_index.try_into().map_err(|_| Error::BadEnum)?)
    }

    #[inline]
    fn serialize_newtype_struct<T>(self, _name: &'static str, value: &T) -> Result<(), Error>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(self)
    }

    #[inline]
    fn serialize_newtype_variant<T>(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _value: &T,
    ) -> Result<(), Error>
    where
        T: ?Sized + Serialize,
    {
        Err(Error::Unsupported)
    }

    #[inline]
    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq, Error> {
        Ok(self)
    }

    #[inline]
    fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple, Error> {
        Ok(self)
    }

    #[inline]
    fn serialize_tuple_struct(self, _name: &'static str, _len: usize) -> Result<Self::SerializeTupleStruct, Error> {
        Ok(self)
    }

    #[inline]
    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant, Error> {
        Err(Error::Unsupported)
    }

    #[inline]
    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap, Error> {
        Err(Error::Unsupported)
    }

    #[inline]
    fn serialize_struct(self, _name: &'static str, _len: usize) -> Result<Self::SerializeStruct, Error> {
        Ok(self)
    }

    #[inline]
    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant, Error> {
        Err(Error::Unsupported)
    }

    #[inline]
    fn collect_str<T>(self, _value: &T) -> Result<(), Error>
    where
        T: core::fmt::Display + ?Sized,
    {
        Err(Error::Unsupported)
    }
}

impl<S: Storage> ser::SerializeSeq for &'_ mut Serializer<S> {
    type Ok = ();
    type Error = Error;

    #[inline]
    fn serialize_element<T>(&mut self, value: &T) -> Result<(), Error>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(&mut **self)
    }

    // Close the sequence.
    #[inline]
    fn end(self) -> Result<(), Error> {
        Ok(())
    }
}

impl<S: Storage> ser::SerializeTuple for &'_ mut Serializer<S> {
    type Ok = ();
    type Error = Error;

    #[inline]
    fn serialize_element<T>(&mut self, value: &T) -> Result<(), Error>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(&mut **self)
    }

    #[inline]
    fn end(self) -> Result<(), Error> {
        Ok(())
    }
}

impl<S: Storage> ser::SerializeTupleStruct for &'_ mut Serializer<S> {
    type Ok = ();
    type Error = Error;

    #[inline]
    fn serialize_field<T>(&mut self, value: &T) -> Result<(), Error>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(&mut **self)
    }

    #[inline]
    fn end(self) -> Result<(), Error> {
        Ok(())
    }
}

impl<S: Storage> ser::SerializeTupleVariant for &'_ mut Serializer<S> {
    type Ok = ();
    type Error = Error;

    #[inline]
    fn serialize_field<T>(&mut self, value: &T) -> Result<(), Error>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(&mut **self)
    }

    #[inline]
    fn end(self) -> Result<(), Error> {
        Ok(())
    }
}

impl<S: Storage> ser::SerializeMap for &'_ mut Serializer<S> {
    type Ok = ();
    type Error = Error;

    #[inline]
    fn serialize_key<T>(&mut self, key: &T) -> Result<(), Error>
    where
        T: ?Sized + Serialize,
    {
        key.serialize(&mut **self)
    }

    #[inline]
    fn serialize_value<T>(&mut self, value: &T) -> Result<(), Error>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(&mut **self)
    }

    #[inline]
    fn end(self) -> Result<(), Error> {
        Ok(())
    }
}

impl<S: Storage> ser::SerializeStruct for &'_ mut Serializer<S> {
    type Ok = ();
    type Error = Error;

    #[inline]
    fn serialize_field<T>(&mut self, _key: &'static str, value: &T) -> Result<(), Error>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(&mut **self)
    }

    #[inline]
    fn end(self) -> Result<(), Error> {
        Ok(())
    }
}

impl<S: Storage> ser::SerializeStructVariant for &'_ mut Serializer<S> {
    type Ok = ();
    type Error = Error;

    #[inline]
    fn serialize_field<T>(&mut self, _key: &'static str, value: &T) -> Result<(), Error>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(&mut **self)
    }

    #[inline]
    fn end(self) -> Result<(), Error> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use serde::Serialize;

    use super::*;
    use crate::request::Slice;

    #[test]
    fn u8_single() {
        let buf = &mut [0xCDu8; 1];
        let mut ser = Serializer {
            output: Slice::new(buf),
        };
        0x12u8.serialize(&mut ser).unwrap();
        assert_eq!(&[0x12], ser.output.finalize());
    }

    #[test]
    fn u16_single() {
        let buf = &mut [0xCDu8; 2];
        let mut ser = Serializer {
            output: Slice::new(buf),
        };
        0x1234u16.serialize(&mut ser).unwrap();
        assert_eq!(&[0x12, 0x34], ser.output.finalize());
    }

    #[test]
    fn u32_single() {
        let buf = &mut [0xCDu8; 4];
        let mut ser = Serializer {
            output: Slice::new(buf),
        };
        0x1234_5678_u32.serialize(&mut ser).unwrap();
        assert_eq!(&[0x12, 0x34, 0x56, 0x78], ser.output.finalize());
    }

    #[test]
    fn u64_single() {
        let buf = &mut [0xCDu8; 8];
        let mut ser = Serializer {
            output: Slice::new(buf),
        };
        0x1234_5678_90AB_CDEF_u64.serialize(&mut ser).unwrap();
        assert_eq!(&[0x12, 0x34, 0x56, 0x78, 0x90, 0xAB, 0xCD, 0xEF], ser.output.finalize());
    }

    #[test]
    fn u128_single() {
        let buf = &mut [0xCDu8; 16];
        let mut ser = Serializer {
            output: Slice::new(buf),
        };
        0x1234_5678_90AB_CDEF_FEDC_BA09_8765_4321_u128
            .serialize(&mut ser)
            .unwrap();
        assert_eq!(
            &[
                0x12, 0x34, 0x56, 0x78, 0x90, 0xAB, 0xCD, 0xEF, 0xFE, 0xDC, 0xBA, 0x09, 0x87, 0x65, 0x43, 0x21
            ],
            ser.output.finalize()
        );
    }

    #[test]
    fn unsigned_tuple() {
        let buf = &mut [0xCDu8; 31];
        let mut ser = Serializer {
            output: Slice::new(buf),
        };
        (
            0x12u8,
            0x1234u16,
            0x1234_5678_u32,
            0x1234_5678_90AB_CDEF_u64,
            0x1234_5678_90AB_CDEF_FEDC_BA09_8765_4321_u128,
        )
            .serialize(&mut ser)
            .unwrap();
        assert_eq!(
            &[
                0x12, 0x12, 0x34, 0x12, 0x34, 0x56, 0x78, 0x12, 0x34, 0x56, 0x78, 0x90, 0xAB, 0xCD, 0xEF, 0x12, 0x34,
                0x56, 0x78, 0x90, 0xAB, 0xCD, 0xEF, 0xFE, 0xDC, 0xBA, 0x09, 0x87, 0x65, 0x43, 0x21
            ],
            ser.output.finalize()
        );
    }

    #[test]
    fn u8_array() {
        let buf = &mut [0xCDu8; 4];
        let mut ser = Serializer {
            output: Slice::new(buf),
        };
        [0xDEu8, 0xAD, 0xBE, 0xEF].serialize(&mut ser).unwrap();
        assert_eq!(&[0xDE, 0xAD, 0xBE, 0xEF], ser.output.finalize());
    }

    #[test]
    fn u16_array() {
        let buf = &mut [0xCDu8; 4];
        let mut ser = Serializer {
            output: Slice::new(buf),
        };
        [0xDEADu16, 0xBEEF].serialize(&mut ser).unwrap();
        assert_eq!(&[0xDE, 0xAD, 0xBE, 0xEF], ser.output.finalize());
    }

    #[test]
    fn u32_array() {
        let buf = &mut [0xCDu8; 8];
        let mut ser = Serializer {
            output: Slice::new(buf),
        };
        [0xDEAD_BEEF_u32, 0x1234_5678_u32].serialize(&mut ser).unwrap();
        assert_eq!(&[0xDE, 0xAD, 0xBE, 0xEF, 0x12, 0x34, 0x56, 0x78], ser.output.finalize());
    }

    #[test]
    fn u64_array() {
        let buf = &mut [0xCDu8; 16];
        let mut ser = Serializer {
            output: Slice::new(buf),
        };
        [0xDEAD_BEEF_1234_5678_u64, 0xABCD_EF00_1122_3344_u64]
            .serialize(&mut ser)
            .unwrap();
        assert_eq!(
            &[
                0xDE, 0xAD, 0xBE, 0xEF, 0x12, 0x34, 0x56, 0x78, 0xAB, 0xCD, 0xEF, 0x00, 0x11, 0x22, 0x33, 0x44
            ],
            ser.output.finalize()
        );
    }

    #[test]
    fn u128_array() {
        let buf = &mut [0xCDu8; 32];
        let mut ser = Serializer {
            output: Slice::new(buf),
        };
        [
            0xDEAD_BEEF_1234_5678_ABCD_EF00_1122_3344_u128,
            0xABCD_EF00_1122_3344_DEAD_BEEF_1234_5678_u128,
        ]
        .serialize(&mut ser)
        .unwrap();
        assert_eq!(
            &[
                0xDE, 0xAD, 0xBE, 0xEF, 0x12, 0x34, 0x56, 0x78, 0xAB, 0xCD, 0xEF, 0x00, 0x11, 0x22, 0x33, 0x44, 0xAB,
                0xCD, 0xEF, 0x00, 0x11, 0x22, 0x33, 0x44, 0xDE, 0xAD, 0xBE, 0xEF, 0x12, 0x34, 0x56, 0x78,
            ],
            ser.output.finalize()
        );
    }

    #[test]
    fn bool_true() {
        let buf = &mut [0xCDu8; 2];
        let mut ser = Serializer {
            output: Slice::new(buf),
        };
        true.serialize(&mut ser).unwrap();
        assert_eq!(&[0x00, 0x01], ser.output.finalize());
    }

    #[test]
    fn bool_false() {
        let buf = &mut [0xCDu8; 2];
        let mut ser = Serializer {
            output: Slice::new(buf),
        };
        false.serialize(&mut ser).unwrap();
        assert_eq!(&[0x00, 0x00], ser.output.finalize());
    }

    #[test]
    fn unit_variant() {
        #[derive(Serialize, Debug, PartialEq)]
        enum Test {
            _Zero,
            _One,
            Two,
        }

        let buf = &mut [0xCDu8; 2];
        let mut ser = Serializer {
            output: Slice::new(buf),
        };

        Test::Two.serialize(&mut ser).unwrap();
        assert_eq!(&[0x00, 0x02], ser.output.finalize());
    }
}
