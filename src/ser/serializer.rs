use serde::{ser, Serialize};

use crate::{
    error::{Error, Result},
    Command, CRC,
};

pub struct Serializer<'se>(&'se mut [u8]);

impl<'se> Serializer<'se> {
    pub fn new(buf: &'se mut [u8], head: u16, cmd: Command, addr: u16) -> Result<Self> {
        if buf.len() < 8 {
            return Err(Error::SerializeBufferTooSmall);
        }
        if buf.len() > u8::MAX as usize {
            return Err(Error::SerializeBufferTooLarge);
        }
        let head = u16::to_be_bytes(head);
        let addr = u16::to_be_bytes(addr);
        buf[0] = head[0];
        buf[1] = head[1];
        buf[2] = 6;
        buf[3] = cmd as u8;
        buf[4] = addr[0];
        buf[5] = addr[1];
        Ok(Self(buf))
    }

    #[inline]
    pub fn push_byte(&mut self, v: u8) -> Result<()> {
        *self
            .0
            .get_mut(self.0[2] as usize)
            .ok_or(Error::SerializeBufferFull)? = v;
        self.0[2] += 1;
        Ok(())
    }

    pub fn finalize(mut self, crc: bool) -> Result<&'se [u8]> {
        if crc {
            let index = self.0[2] as usize;
            // calculate crc from [CMD] to end.
            let crc = CRC.checksum(&self.0[3..index]).to_le_bytes();
            // CRC should be little endian in payload, so can't use add_u16
            self.push_byte(crc[0])?;
            self.push_byte(crc[1])?;
        }
        let index = self.0[2] as usize;
        self.0[2] -= 3; //[LEN:1] -> first 3 bytes are excluded
        Ok(&self.0[..index])
    }
}

trait SerializeBigEndian<T> {
    fn serialize_be(&mut self, data: T) -> Result<()>;
}

// Macro for blanket implementation of primitive type serialization
macro_rules! impl_serialize_be {
    ($($ty:ident)+) => ($(
        impl SerializeBigEndian<$ty> for Serializer<'_> {
            #[inline]
            fn serialize_be(&mut self, v: $ty) -> Result<()> {
                let bytes = v.to_be_bytes();
                for byte in bytes {
                    self.push_byte(byte)?;
                }
                Ok(())
            }
        }
    )+)
}

impl_serialize_be! { u16 i16 u32 i32 u64 i64 f32 f64 }

impl ser::Serializer for &'_ mut Serializer<'_> {
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
    fn serialize_i8(self, _v: i8) -> Result<()> {
        Err(Error::NotYetImplemented)
    }

    #[inline]
    fn serialize_i16(self, v: i16) -> Result<()> {
        self.serialize_be(v)
    }

    #[inline]
    fn serialize_i32(self, v: i32) -> Result<()> {
        self.serialize_be(v)
    }

    #[inline]
    fn serialize_i64(self, v: i64) -> Result<()> {
        self.serialize_be(v)
    }

    #[inline]
    fn serialize_i128(self, _v: i128) -> Result<()> {
        Err(Error::NotYetImplemented)
    }

    #[inline]
    fn serialize_u8(self, _v: u8) -> Result<()> {
        Err(Error::NotYetImplemented)
    }

    #[inline]
    fn serialize_u16(self, v: u16) -> Result<()> {
        self.serialize_be(v)
    }

    #[inline]
    fn serialize_u32(self, v: u32) -> Result<()> {
        self.serialize_be(v)
    }

    #[inline]
    fn serialize_u64(self, v: u64) -> Result<()> {
        self.serialize_be(v)
    }

    #[inline]
    fn serialize_u128(self, _v: u128) -> Result<()> {
        Err(Error::WontImplement)
    }

    #[inline]
    fn serialize_f32(self, v: f32) -> Result<()> {
        self.serialize_be(v)
    }

    #[inline]
    fn serialize_f64(self, v: f64) -> Result<()> {
        self.serialize_be(v)
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
    fn serialize_bytes(self, _v: &[u8]) -> Result<()> {
        Err(Error::NotYetImplemented)
    }

    #[inline]
    fn serialize_none(self) -> Result<()> {
        Err(Error::NotYetImplemented)
    }

    #[inline]
    fn serialize_some<T>(self, _value: &T) -> Result<()>
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
        self.serialize_be(variant_index as u16)
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
        self.serialize_be(variant_index as u16)?;
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
        self.serialize_be(variant_index as u16)?;
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
        self.serialize_be(variant_index as u16)?;
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

impl ser::SerializeSeq for &'_ mut Serializer<'_> {
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

impl ser::SerializeTuple for &'_ mut Serializer<'_> {
    type Ok = ();
    type Error = Error;

    fn serialize_element<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<()> {
        Ok(())
    }
}

impl ser::SerializeTupleStruct for &'_ mut Serializer<'_> {
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

impl ser::SerializeTupleVariant for &'_ mut Serializer<'_> {
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

impl ser::SerializeMap for &'_ mut Serializer<'_> {
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

impl ser::SerializeStruct for &'_ mut Serializer<'_> {
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

impl ser::SerializeStructVariant for &'_ mut Serializer<'_> {
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
