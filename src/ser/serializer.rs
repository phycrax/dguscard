use crate::{
    error::{Error, Result},
    ser::output::Output,
    Command,
};
use serde::{ser, Serialize};

pub(crate) struct Serializer<O: Output> {
    pub output: O,
}

impl<O: Output> Serializer<O> {
    pub fn init(&mut self, hdr: u16, cmd: Command, addr: u16) -> Result<()> {
        self.serialize_be(hdr)?;
        self.serialize_be(cmd as u16)?;
        self.serialize_be(addr)?;
        Ok(())
    }

    pub fn finalize(mut self, crc: Option<crc::Digest<'_, u16>>) -> Result<O::Out> {
        if let Some(mut digest) = crc {
            digest.update(&self.output.as_bytes()[3..]);
            let crc = u16::to_le_bytes(digest.finalize());
            self.output.try_push(crc[0])?;
            self.output.try_push(crc[1])?;
        }
        Ok(self.output.finalize())
    }
}

trait SerializeBigEndian<T> {
    fn serialize_be(&mut self, data: T) -> Result<()>;
}

// Macro for blanket implementation for primitive type big endian serialization
macro_rules! impl_serialize_be {
    ($($ty:ident)+) => ($(
        impl<O: Output> SerializeBigEndian<$ty> for Serializer<O> {
            #[inline]
            fn serialize_be(&mut self, v: $ty) -> Result<()> {
                let bytes = v.to_be_bytes();
                for byte in bytes {
                    self.output.try_push(byte)?;
                }
                Ok(())
            }
        }
    )+)
}

impl_serialize_be! { u16 i16 u32 i32 u64 i64 f32 f64 }

impl<O: Output> ser::Serializer for &'_ mut Serializer<O> {
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

impl<O: Output> ser::SerializeSeq for &'_ mut Serializer<O> {
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

impl<O: Output> ser::SerializeTuple for &'_ mut Serializer<O> {
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

impl<O: Output> ser::SerializeTupleStruct for &'_ mut Serializer<O> {
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

impl<O: Output> ser::SerializeTupleVariant for &'_ mut Serializer<O> {
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

impl<O: Output> ser::SerializeMap for &'_ mut Serializer<O> {
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

impl<O: Output> ser::SerializeStruct for &'_ mut Serializer<O> {
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

impl<O: Output> ser::SerializeStructVariant for &'_ mut Serializer<O> {
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
