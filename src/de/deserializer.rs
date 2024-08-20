use crate::{
    error::{Error, Result},
    Config,
};
use serde::de::{self, DeserializeSeed, Visitor};

/// `serde` compatible deserializer.
pub struct Deserializer<'de>(&'de [u8]);

impl<'de> Deserializer<'de> {
    /// Obtain a Deserializer from an already parsed slice of bytes
    pub fn from_bytes(s: &'de [u8]) -> Self {
        Self(s)
    }

    /// Return the remaining (unused) bytes in the Deserializer
    pub fn finalize(self) -> &'de [u8] {
        self.0
    }
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
                let (bytes, rest) = self.0.split_first_chunk().ok_or(Error::DeserializeUnexpectedEnd)?;
                self.0 = rest;
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
        let v = self.deserialize_be()?;
        visitor.visit_i8(v)
    }

    #[inline]
    fn deserialize_i16<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let v = self.deserialize_be()?;
        visitor.visit_i16(v)
    }

    #[inline]
    fn deserialize_i32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let v = self.deserialize_be()?;
        visitor.visit_i32(v)
    }

    #[inline]
    fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let v = self.deserialize_be()?;
        visitor.visit_i64(v)
    }

    #[inline]
    fn deserialize_i128<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let v = self.deserialize_be()?;
        visitor.visit_i128(v)
    }

    #[inline]
    fn deserialize_u8<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let v = self.deserialize_be()?;
        visitor.visit_u8(v)
    }

    #[inline]
    fn deserialize_u16<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let v = self.deserialize_be()?;
        visitor.visit_u16(v)
    }

    #[inline]
    fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let v = self.deserialize_be()?;
        visitor.visit_u32(v)
    }

    #[inline]
    fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let v = self.deserialize_be()?;
        visitor.visit_u64(v)
    }

    #[inline]
    fn deserialize_u128<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let v = self.deserialize_be()?;
        visitor.visit_u128(v)
    }

    #[inline]
    fn deserialize_f32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let v = self.deserialize_be()?;
        visitor.visit_f32(v)
    }

    #[inline]
    fn deserialize_f64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let v = self.deserialize_be()?;
        visitor.visit_f64(v)
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
        _visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        Err(Error::NotYetImplemented)
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
