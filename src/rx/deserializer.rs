use crate::error::{Error, Result};
use serde::de::{self, DeserializeSeed, IntoDeserializer, Visitor};

/// `serde` compatible deserializer.
pub struct Deserializer<'de> {
    pub input: &'de [u8],
}

// Generic trait for blanket impl of big endian deserialization
trait DeserializeBigEndian<T> {
    fn deserialize_be(&mut self) -> Result<T>;
}

// Big endian deserialization macro
macro_rules! impl_deserialize_be{
    ($($ty:ident)+) => ($(
        impl DeserializeBigEndian<$ty> for Deserializer<'_> {
            #[inline]
            fn deserialize_be(&mut self) -> Result<$ty> {
                let (bytes, rest) = self.input.split_first_chunk().ok_or(Error::DeserializeUnexpectedEnd)?;
                self.input = rest;
                Ok($ty::from_be_bytes(*bytes))
            }
        }
    )+)
}

// Deserialize following types with the macro
impl_deserialize_be! { u8 u16 u32 u64 u128 i8 i16 i32 i64 i128 f32 f64 }

/// Serde deserializer implementation
impl<'de> de::Deserializer<'de> for &'_ mut Deserializer<'de> {
    type Error = Error;

    #[inline]
    fn is_human_readable(&self) -> bool {
        false
    }

    #[inline]
    fn deserialize_any<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        Err(Error::WontImplement)
    }

    #[inline]
    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        // Take a boolean encoded as u16
        let v: u16 = self.deserialize_be()?;
        let v = match v {
            0 => false,
            1 => true,
            _ => return Err(Error::DeserializeBadBool),
        };
        visitor.visit_bool(v)
    }

    #[inline]
    fn deserialize_i8<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_i8(self.deserialize_be()?)
    }

    #[inline]
    fn deserialize_i16<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_i16(self.deserialize_be()?)
    }

    #[inline]
    fn deserialize_i32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_i32(self.deserialize_be()?)
    }

    #[inline]
    fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_i64(self.deserialize_be()?)
    }

    #[inline]
    fn deserialize_i128<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_i128(self.deserialize_be()?)
    }

    #[inline]
    fn deserialize_u8<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_u8(self.deserialize_be()?)
    }

    #[inline]
    fn deserialize_u16<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_u16(self.deserialize_be()?)
    }

    #[inline]
    fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_u32(self.deserialize_be()?)
    }

    #[inline]
    fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_u64(self.deserialize_be()?)
    }

    #[inline]
    fn deserialize_u128<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_u128(self.deserialize_be()?)
    }

    #[inline]
    fn deserialize_f32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_f32(self.deserialize_be()?)
    }

    #[inline]
    fn deserialize_f64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_f64(self.deserialize_be()?)
    }

    #[inline]
    fn deserialize_char<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        Err(Error::NotYetImplemented)
    }

    #[inline]
    fn deserialize_str<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        Err(Error::NotYetImplemented)
    }

    #[inline]
    fn deserialize_string<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        Err(Error::NotYetImplemented)
    }

    #[inline]
    fn deserialize_bytes<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        Err(Error::NotYetImplemented)
    }

    #[inline]
    fn deserialize_byte_buf<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        Err(Error::NotYetImplemented)
    }

    #[inline]
    fn deserialize_option<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        Err(Error::NotYetImplemented)
    }

    #[inline]
    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_unit()
    }

    #[inline]
    fn deserialize_unit_struct<V>(self, _name: &'static str, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_unit(visitor)
    }

    #[inline]
    fn deserialize_newtype_struct<V>(self, _name: &'static str, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_newtype_struct(self)
    }

    #[inline]
    fn deserialize_seq<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        Err(Error::NotYetImplemented)
    }

    #[inline]
    fn deserialize_tuple<V>(self, len: usize, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_seq(SeqAccess {
            deserializer: self,
            len,
        })
    }

    #[inline]
    fn deserialize_tuple_struct<V>(
        self,
        _name: &'static str,
        len: usize,
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_tuple(len, visitor)
    }

    #[inline]
    fn deserialize_map<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        Err(Error::NotYetImplemented)
    }

    #[inline]
    fn deserialize_struct<V>(
        self,
        _name: &'static str,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_tuple(fields.len(), visitor)
    }

    #[inline]
    fn deserialize_enum<V>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_enum(self)
    }

    #[inline]
    fn deserialize_identifier<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        Err(Error::WontImplement)
    }

    #[inline]
    fn deserialize_ignored_any<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        Err(Error::WontImplement)
    }
}

impl<'de> serde::de::VariantAccess<'de> for &'_ mut Deserializer<'de> {
    type Error = Error;

    #[inline]
    fn unit_variant(self) -> Result<()> {
        Ok(())
    }

    #[inline]
    fn newtype_variant_seed<V: DeserializeSeed<'de>>(self, seed: V) -> Result<V::Value> {
        DeserializeSeed::deserialize(seed, self)
    }

    #[inline]
    fn tuple_variant<V: Visitor<'de>>(self, len: usize, visitor: V) -> Result<V::Value> {
        serde::de::Deserializer::deserialize_tuple(self, len, visitor)
    }

    #[inline]
    fn struct_variant<V: Visitor<'de>>(
        self,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value> {
        serde::de::Deserializer::deserialize_tuple(self, fields.len(), visitor)
    }
}

impl<'de> serde::de::EnumAccess<'de> for &'_ mut Deserializer<'de> {
    type Error = Error;
    type Variant = Self;

    #[inline]
    fn variant_seed<V: DeserializeSeed<'de>>(self, seed: V) -> Result<(V::Value, Self)> {
        let v: u16 = self.deserialize_be()?;
        let v = DeserializeSeed::deserialize(seed, v.into_deserializer())?;
        Ok((v, self))
    }
}

struct SeqAccess<'a, 'b: 'a> {
    deserializer: &'a mut Deserializer<'b>,
    len: usize,
}

impl<'a, 'b: 'a> serde::de::SeqAccess<'b> for SeqAccess<'a, 'b> {
    type Error = Error;

    #[inline]
    fn next_element_seed<V: DeserializeSeed<'b>>(&mut self, seed: V) -> Result<Option<V::Value>> {
        if self.len > 0 {
            self.len -= 1;
            Ok(Some(DeserializeSeed::deserialize(
                seed,
                &mut *self.deserializer,
            )?))
        } else {
            Ok(None)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;

    #[test]
    fn u8_single() {
        let input = &[0xDB];
        let mut de = Deserializer { input };
        assert_eq!(0xDB, u8::deserialize(&mut de).unwrap());
    }

    #[test]
    fn u16_single() {
        let input = &[0xDE, 0xBE];
        let mut de = Deserializer { input };
        assert_eq!(0xDEBE, u16::deserialize(&mut de).unwrap());
    }

    #[test]
    fn u32_single() {
        let input = &[0xDE, 0xAD, 0xBE, 0xEF];
        let mut de = Deserializer { input };
        assert_eq!(0xDEADBEEF, u32::deserialize(&mut de).unwrap());
        assert!(de.input.is_empty());
    }

    #[test]
    fn u64_single() {
        let input = &[0xDE, 0xAD, 0xBE, 0xEF, 0xFE, 0xEB, 0xDA, 0xED];
        let mut de = Deserializer { input };
        assert_eq!(0xDEADBEEFFEEBDAED, u64::deserialize(&mut de).unwrap());
        assert!(de.input.is_empty());
    }

    #[test]
    fn u128_single() {
        let input = &[
            0xDE, 0xAD, 0xBE, 0xEF, 0xFE, 0xEB, 0xDA, 0xED, 0x12, 0x23, 0x34, 0x45, 0x56, 0x67,
            0x78, 0x89,
        ];
        let mut de = Deserializer { input };
        assert_eq!(
            0xDEADBEEFFEEBDAED1223344556677889,
            u128::deserialize(&mut de).unwrap()
        );
        assert!(de.input.is_empty());
    }

    #[test]
    fn unsigned_tuple() {
        let input = &[
            0xDE, 0xAD, 0xBE, 0xEF, 0xFE, 0xEB, 0xDA, 0xED, 0x12, 0x23, 0x34, 0x45, 0x56, 0x67,
            0x78, 0x89, 0x10, 0x44, 0x33, 0x22, 0x11, 0x88, 0x77, 0x66, 0x55, 0x44, 0x33, 0x22,
            0x11, 0xDE, 0xAD,
        ];
        let mut de = Deserializer { input };
        type TestTuple = (u128, u8, u32, u64, u16);
        assert_eq!(
            (
                0xDEADBEEFFEEBDAED1223344556677889u128,
                0x10u8,
                0x44332211u32,
                0x8877665544332211u64,
                0xDEADu16
            ),
            TestTuple::deserialize(&mut de).unwrap()
        );
        assert!(de.input.is_empty());
    }

    #[test]
    fn u8_array() {
        let input = &[0xDE, 0xAD, 0xBE, 0xEF];
        let mut de = Deserializer { input };
        type TestArray = [u8; 4];
        assert_eq!(
            [0xDE, 0xAD, 0xBE, 0xEF],
            TestArray::deserialize(&mut de).unwrap()
        );
        assert!(de.input.is_empty());
    }

    #[test]
    fn u16_array() {
        let input = &[
            0xDE, 0xAD, 0xBE, 0xEF, 0x12, 0x34, 0x56, 0x78, 0xFE, 0x12, 0xCD, 0x34,
        ];
        let mut de = Deserializer { input };
        type TestArray = [u16; 6];
        assert_eq!(
            [0xDEAD, 0xBEEF, 0x1234, 0x5678, 0xFE12, 0xCD34],
            TestArray::deserialize(&mut de).unwrap()
        );
        assert!(de.input.is_empty());
    }

    #[test]
    fn u32_array() {
        let input = &[
            0xDE, 0xAD, 0xBE, 0xEF, 0x12, 0x34, 0x56, 0x78, 0xFE, 0x12, 0xCD, 0x34,
        ];
        let mut de = Deserializer { input };
        type TestArray = [u32; 3];
        assert_eq!(
            [0xDEADBEEF, 0x12345678, 0xFE12CD34],
            TestArray::deserialize(&mut de).unwrap()
        );
        assert!(de.input.is_empty());
    }

    #[test]
    fn bool_true() {
        let input = &[0x00, 0x01];
        let mut de = Deserializer { input };
        assert!(bool::deserialize(&mut de).unwrap());
        assert!(de.input.is_empty());
    }

    #[test]
    fn bool_false() {
        let input = &[0x00, 0x00];
        let mut de = Deserializer { input };
        assert!(!bool::deserialize(&mut de).unwrap());
        assert!(de.input.is_empty());
    }

    #[test]
    fn bool_bad() {
        let input = &[0x01, 0x00];
        let mut de = Deserializer { input };
        assert_eq!(Err(Error::DeserializeBadBool), bool::deserialize(&mut de));
        assert!(de.input.is_empty());
    }

    #[test]
    fn unit_variant() {
        #[derive(Deserialize, Debug, PartialEq)]
        enum Test {
            Zero,
            One,
            Two,
        }

        let input = &[0x00, 0x02];
        let mut de = Deserializer { input };
        assert_eq!(Ok(Test::Two), Test::deserialize(&mut de));
        assert!(de.input.is_empty());
    }

    #[test]
    fn newtype_variant() {
        #[derive(Deserialize, Debug, PartialEq)]
        enum Test {
            Zero(u16),
            One(u16),
            Two(u16),
        }

        let input = &[0x00, 0x01, 0x12, 0x34];
        let mut de = Deserializer { input };
        assert_eq!(Ok(Test::One(0x1234)), Test::deserialize(&mut de));
        assert!(de.input.is_empty());
    }
}
