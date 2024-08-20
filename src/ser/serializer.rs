use crate::{
    error::{Error, Result},
    ser::storage::Storage,
    Command,
};
use serde::{ser, Serialize};

/// `serde` compatible serializer.
///
/// Serialization output type is generic and must implement the [`Storage`] trait.
/// Unless you are implementing [`Storage`], you don't have to use this directly.
pub struct Serializer<S: Storage> {
    output: S,
}

impl<S: Storage> Serializer<S> {
    /// Create a new serializer with the given output type, header, command and address
    pub fn new(output: S, header: u16, cmd: Command, addr: u16) -> Result<Self> {
        let mut this = Self { output };
        ser::Serializer::serialize_u16(&mut this, header)?;
        ser::Serializer::serialize_u16(&mut this, cmd as u16)?;
        ser::Serializer::serialize_u16(&mut this, addr)?;
        Ok(this)
    }

    /// Finalize the serialization process with optional CRC
    pub fn finalize(mut self, crc: Option<crc::Digest<'_, u16>>) -> Result<S::Output> {
        if let Some(mut digest) = crc {
            digest.update(&self.output[3..]);
            ser::Serializer::serialize_u16(&mut self, digest.finalize().swap_bytes())?;
        }
        self.output[2] = (self.output.len() - 3) as u8;
        Ok(self.output.finalize())
    }
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
        self.serialize_u16(variant_index as u16)
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
        self.serialize_u16(variant_index as u16)?;
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
        self.serialize_u16(variant_index as u16)?;
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
        self.serialize_u16(variant_index as u16)?;
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
