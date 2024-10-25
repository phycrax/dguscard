use super::Storage;
use crate::{Error, Result};
use serde::{ser, Serialize};

/// `serde` compatible serializer.
///
/// Serialization output type is generic and must implement the [`Storage`] trait.
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
    fn serialize_bool(self, v: bool) -> Result<()> {
        self.serialize_u16(if v { 1 } else { 0 })
    }

    #[inline]
    fn serialize_i8(self, v: i8) -> Result<()> {
        self.output.try_extend(&v.to_be_bytes())
    }

    #[inline]
    fn serialize_i16(self, v: i16) -> Result<()> {
        self.output.try_extend(&v.to_be_bytes())
    }

    #[inline]
    fn serialize_i32(self, v: i32) -> Result<()> {
        self.output.try_extend(&v.to_be_bytes())
    }

    #[inline]
    fn serialize_i64(self, v: i64) -> Result<()> {
        self.output.try_extend(&v.to_be_bytes())
    }

    #[inline]
    fn serialize_i128(self, v: i128) -> Result<()> {
        self.output.try_extend(&v.to_be_bytes())
    }

    #[inline]
    fn serialize_u8(self, v: u8) -> Result<()> {
        self.output.try_extend(&v.to_be_bytes())
    }

    #[inline]
    fn serialize_u16(self, v: u16) -> Result<()> {
        self.output.try_extend(&v.to_be_bytes())
    }

    #[inline]
    fn serialize_u32(self, v: u32) -> Result<()> {
        self.output.try_extend(&v.to_be_bytes())
    }

    #[inline]
    fn serialize_u64(self, v: u64) -> Result<()> {
        self.output.try_extend(&v.to_be_bytes())
    }

    #[inline]
    fn serialize_u128(self, v: u128) -> Result<()> {
        self.output.try_extend(&v.to_be_bytes())
    }

    #[inline]
    fn serialize_f32(self, v: f32) -> Result<()> {
        self.output.try_extend(&v.to_be_bytes())
    }

    #[inline]
    fn serialize_f64(self, v: f64) -> Result<()> {
        self.output.try_extend(&v.to_be_bytes())
    }

    #[inline]
    fn serialize_char(self, _v: char) -> Result<()> {
        Err(Error::NotYetImplemented)
    }

    #[inline]
    fn serialize_str(self, _v: &str) -> Result<()> {
        Err(Error::NotYetImplemented)
    }

    #[inline]
    fn serialize_bytes(self, v: &[u8]) -> Result<()> {
        self.output.try_extend(v)
    }

    #[inline]
    fn serialize_none(self) -> Result<()> {
        Err(Error::NotYetImplemented)
    }

    #[inline]
    fn serialize_some<T>(self, _v: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        Err(Error::NotYetImplemented)
    }

    #[inline]
    fn serialize_unit(self) -> Result<()> {
        Ok(())
    }

    #[inline]
    fn serialize_unit_struct(self, _name: &'static str) -> Result<()> {
        Ok(())
    }

    #[inline]
    fn serialize_unit_variant(
        self,
        _name: &'static str,
        variant_index: u32,
        _variant: &'static str,
    ) -> Result<()> {
        self.serialize_u16(
            variant_index
                .try_into()
                .map_err(|_| Error::SerializeBadEnum)?,
        )
    }

    #[inline]
    fn serialize_newtype_struct<T>(self, _name: &'static str, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(self)
    }

    #[inline]
    fn serialize_newtype_variant<T>(
        self,
        _name: &'static str,
        variant_index: u32,
        _variant: &'static str,
        value: &T,
    ) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        self.serialize_u16(
            variant_index
                .try_into()
                .map_err(|_| Error::SerializeBadEnum)?,
        )?;
        value.serialize(self)
    }

    #[inline]
    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq> {
        Err(Error::NotYetImplemented)
    }

    #[inline]
    fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple> {
        Ok(self)
    }

    #[inline]
    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleStruct> {
        Ok(self)
    }

    #[inline]
    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant> {
        self.serialize_u16(
            variant_index
                .try_into()
                .map_err(|_| Error::SerializeBadEnum)?,
        )?;
        Ok(self)
    }

    #[inline]
    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap> {
        Err(Error::NotYetImplemented)
    }

    #[inline]
    fn serialize_struct(self, _name: &'static str, _len: usize) -> Result<Self::SerializeStruct> {
        Ok(self)
    }

    #[inline]
    fn serialize_struct_variant(
        self,
        _name: &'static str,
        variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant> {
        self.serialize_u16(
            variant_index
                .try_into()
                .map_err(|_| Error::SerializeBadEnum)?,
        )?;
        Ok(self)
    }

    #[inline]
    fn collect_str<T>(self, _value: &T) -> Result<()>
    where
        T: core::fmt::Display + ?Sized,
    {
        Err(Error::NotYetImplemented)
    }
}

impl<S: Storage> ser::SerializeSeq for &'_ mut Serializer<S> {
    type Ok = ();
    type Error = Error;

    #[inline]
    fn serialize_element<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(&mut **self)
    }

    // Close the sequence.
    #[inline]
    fn end(self) -> Result<()> {
        Ok(())
    }
}

impl<S: Storage> ser::SerializeTuple for &'_ mut Serializer<S> {
    type Ok = ();
    type Error = Error;

    #[inline]
    fn serialize_element<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(&mut **self)
    }

    #[inline]
    fn end(self) -> Result<()> {
        Ok(())
    }
}

impl<S: Storage> ser::SerializeTupleStruct for &'_ mut Serializer<S> {
    type Ok = ();
    type Error = Error;

    #[inline]
    fn serialize_field<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(&mut **self)
    }

    #[inline]
    fn end(self) -> Result<()> {
        Ok(())
    }
}

impl<S: Storage> ser::SerializeTupleVariant for &'_ mut Serializer<S> {
    type Ok = ();
    type Error = Error;

    #[inline]
    fn serialize_field<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(&mut **self)
    }

    #[inline]
    fn end(self) -> Result<()> {
        Ok(())
    }
}

impl<S: Storage> ser::SerializeMap for &'_ mut Serializer<S> {
    type Ok = ();
    type Error = Error;

    #[inline]
    fn serialize_key<T>(&mut self, key: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        key.serialize(&mut **self)
    }

    #[inline]
    fn serialize_value<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(&mut **self)
    }

    #[inline]
    fn end(self) -> Result<()> {
        Ok(())
    }
}

impl<S: Storage> ser::SerializeStruct for &'_ mut Serializer<S> {
    type Ok = ();
    type Error = Error;

    #[inline]
    fn serialize_field<T>(&mut self, _key: &'static str, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(&mut **self)
    }

    #[inline]
    fn end(self) -> Result<()> {
        Ok(())
    }
}

impl<S: Storage> ser::SerializeStructVariant for &'_ mut Serializer<S> {
    type Ok = ();
    type Error = Error;

    #[inline]
    fn serialize_field<T>(&mut self, _key: &'static str, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(&mut **self)
    }

    #[inline]
    fn end(self) -> Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::request::Slice;
    use serde::Serialize;

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
        0x12345678u32.serialize(&mut ser).unwrap();
        assert_eq!(&[0x12, 0x34, 0x56, 0x78], ser.output.finalize());
    }

    #[test]
    fn u64_single() {
        let buf = &mut [0xCDu8; 8];
        let mut ser = Serializer {
            output: Slice::new(buf),
        };
        0x1234567890ABCDEFu64.serialize(&mut ser).unwrap();
        assert_eq!(
            &[0x12, 0x34, 0x56, 0x78, 0x90, 0xAB, 0xCD, 0xEF],
            ser.output.finalize()
        );
    }

    #[test]
    fn u128_single() {
        let buf = &mut [0xCDu8; 16];
        let mut ser = Serializer {
            output: Slice::new(buf),
        };
        0x1234567890ABCDEFFEDCBA0987654321u128
            .serialize(&mut ser)
            .unwrap();
        assert_eq!(
            &[
                0x12, 0x34, 0x56, 0x78, 0x90, 0xAB, 0xCD, 0xEF, 0xFE, 0xDC, 0xBA, 0x09, 0x87, 0x65,
                0x43, 0x21
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
            0x12345678u32,
            0x1234567890ABCDEFu64,
            0x1234567890ABCDEFFEDCBA0987654321u128,
        )
            .serialize(&mut ser)
            .unwrap();
        assert_eq!(
            &[
                0x12, 0x12, 0x34, 0x12, 0x34, 0x56, 0x78, 0x12, 0x34, 0x56, 0x78, 0x90, 0xAB, 0xCD,
                0xEF, 0x12, 0x34, 0x56, 0x78, 0x90, 0xAB, 0xCD, 0xEF, 0xFE, 0xDC, 0xBA, 0x09, 0x87,
                0x65, 0x43, 0x21
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
        [0xDEADBEEFu32, 0x12345678].serialize(&mut ser).unwrap();
        assert_eq!(
            &[0xDE, 0xAD, 0xBE, 0xEF, 0x12, 0x34, 0x56, 0x78],
            ser.output.finalize()
        );
    }

    #[test]
    fn u64_array() {
        let buf = &mut [0xCDu8; 16];
        let mut ser = Serializer {
            output: Slice::new(buf),
        };
        [0xDEADBEEF12345678u64, 0xABCDEF0011223344u64]
            .serialize(&mut ser)
            .unwrap();
        assert_eq!(
            &[
                0xDE, 0xAD, 0xBE, 0xEF, 0x12, 0x34, 0x56, 0x78, 0xAB, 0xCD, 0xEF, 0x00, 0x11, 0x22,
                0x33, 0x44
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
            0xDEADBEEF12345678ABCDEF0011223344u128,
            0xABCDEF0011223344DEADBEEF12345678,
        ]
        .serialize(&mut ser)
        .unwrap();
        assert_eq!(
            &[
                0xDE, 0xAD, 0xBE, 0xEF, 0x12, 0x34, 0x56, 0x78, 0xAB, 0xCD, 0xEF, 0x00, 0x11, 0x22,
                0x33, 0x44, 0xAB, 0xCD, 0xEF, 0x00, 0x11, 0x22, 0x33, 0x44, 0xDE, 0xAD, 0xBE, 0xEF,
                0x12, 0x34, 0x56, 0x78,
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
        let buf = &mut [0xCDu8; 2];
        let mut ser = Serializer {
            output: Slice::new(buf),
        };

        #[derive(Serialize, Debug, PartialEq)]
        enum Test {
            _Zero,
            _One,
            Two,
        }

        Test::Two.serialize(&mut ser).unwrap();
        assert_eq!(&[0x00, 0x02], ser.output.finalize());
    }

    #[test]
    fn newtype_variant() {
        let buf = &mut [0xCDu8; 4];
        let mut ser = Serializer {
            output: Slice::new(buf),
        };

        #[derive(Serialize, Debug, PartialEq)]
        enum Test {
            _Zero(u16),
            One(u16),
            _Two(u16),
        }

        Test::One(0x1234).serialize(&mut ser).unwrap();
        assert_eq!(&[0x00, 0x01, 0x12, 0x34], ser.output.finalize());
    }
}
