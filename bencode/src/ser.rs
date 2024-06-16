use std::collections::BTreeMap;

use anyhow::anyhow;
use bstr::BString;
use serde::{ser, Serialize};

use super::{error::Error, BencodeValue};

pub(super) struct Serializer;

impl ser::Serializer for Serializer {
    type Ok = BencodeValue;
    type Error = Error;

    type SerializeSeq = SeqSerializer;
    type SerializeTuple = SeqSerializer;
    type SerializeTupleStruct = SeqSerializer;
    type SerializeTupleVariant = TupleVariantSerializer;
    type SerializeMap = MapSerializer;
    type SerializeStruct = MapSerializer;
    type SerializeStructVariant = StructVariantSerializer;

    fn serialize_bool(self, v: bool) -> Result<Self::Ok, Self::Error> {
        self.serialize_i64(i64::from(v))
    }

    fn serialize_i8(self, v: i8) -> Result<Self::Ok, Self::Error> {
        self.serialize_i64(v as i64)
    }

    fn serialize_i16(self, v: i16) -> Result<Self::Ok, Self::Error> {
        self.serialize_i64(v as i64)
    }

    fn serialize_i32(self, v: i32) -> Result<Self::Ok, Self::Error> {
        self.serialize_i64(v as i64)
    }

    fn serialize_i64(self, v: i64) -> Result<Self::Ok, Self::Error> {
        Ok(BencodeValue::Integer(v))
    }

    fn serialize_u8(self, v: u8) -> Result<Self::Ok, Self::Error> {
        self.serialize_u64(v as u64)
    }

    fn serialize_u16(self, v: u16) -> Result<Self::Ok, Self::Error> {
        self.serialize_u64(v as u64)
    }

    fn serialize_u32(self, v: u32) -> Result<Self::Ok, Self::Error> {
        self.serialize_u64(v as u64)
    }

    fn serialize_u64(self, v: u64) -> Result<Self::Ok, Self::Error> {
        match i64::try_from(v).ok() {
            Some(v) => self.serialize_i64(v),
            None => Err(anyhow!("invalid value: value in u64 cannot be stored in i64").into()),
        }
    }

    fn serialize_f32(self, _v: f32) -> Result<Self::Ok, Self::Error> {
        Err(anyhow!("unsupported type: f32").into())
    }

    fn serialize_f64(self, _v: f64) -> Result<Self::Ok, Self::Error> {
        Err(anyhow!("unsupported type: f64").into())
    }

    fn serialize_char(self, v: char) -> Result<Self::Ok, Self::Error> {
        self.serialize_str(v.encode_utf8(&mut [0u8; 4]))
    }

    fn serialize_str(self, v: &str) -> Result<Self::Ok, Self::Error> {
        self.serialize_bytes(v.as_bytes())
    }

    fn serialize_bytes(self, v: &[u8]) -> Result<Self::Ok, Self::Error> {
        Ok(BencodeValue::String(BString::new(v.to_vec())))
    }

    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        Err(anyhow!("unsupported type: none").into())
    }

    fn serialize_some<T: ?Sized>(self, value: &T) -> Result<Self::Ok, Self::Error>
    where
        T: serde::Serialize,
    {
        value.serialize(self)
    }

    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
        Err(anyhow!("unsupported type: unit").into())
    }

    fn serialize_unit_struct(self, _name: &'static str) -> Result<Self::Ok, Self::Error> {
        Err(anyhow!("unsupported type: unit struct").into())
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
    ) -> Result<Self::Ok, Self::Error> {
        self.serialize_str(variant)
    }

    fn serialize_newtype_struct<T: ?Sized>(
        self,
        _name: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: serde::Serialize,
    {
        value.serialize(self)
    }

    fn serialize_newtype_variant<T: ?Sized>(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: serde::Serialize,
    {
        let mut dict = BTreeMap::new();
        dict.insert(variant.to_string(), value.serialize(self)?);
        Ok(BencodeValue::Dict(dict))
    }

    fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        Ok(SeqSerializer {
            result: match len {
                Some(len) => Vec::with_capacity(len),
                None => Vec::new(),
            },
        })
    }

    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple, Self::Error> {
        Ok(SeqSerializer {
            result: Vec::with_capacity(len),
        })
    }

    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        Ok(SeqSerializer {
            result: Vec::with_capacity(len),
        })
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        Ok(TupleVariantSerializer {
            name: variant.to_string(),
            result: Vec::with_capacity(len),
        })
    }

    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        Ok(MapSerializer {
            result: BTreeMap::new(),
            next_key: None,
        })
    }

    fn serialize_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        Ok(MapSerializer {
            result: BTreeMap::new(),
            next_key: None,
        })
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        Ok(StructVariantSerializer {
            name: variant.to_string(),
            result: BTreeMap::new(),
        })
    }
}

pub(super) struct SeqSerializer {
    result: Vec<BencodeValue>,
}

impl ser::SerializeSeq for SeqSerializer {
    type Ok = BencodeValue;
    type Error = Error;

    fn serialize_element<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: serde::Serialize,
    {
        self.result.push(value.serialize(Serializer)?);
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(BencodeValue::List(self.result.into_boxed_slice()))
    }
}

impl ser::SerializeTuple for SeqSerializer {
    type Ok = BencodeValue;
    type Error = Error;

    fn serialize_element<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: serde::Serialize,
    {
        ser::SerializeSeq::serialize_element(self, value)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        ser::SerializeSeq::end(self)
    }
}

impl ser::SerializeTupleStruct for SeqSerializer {
    type Ok = BencodeValue;
    type Error = Error;

    fn serialize_field<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: serde::Serialize,
    {
        ser::SerializeSeq::serialize_element(self, value)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        ser::SerializeSeq::end(self)
    }
}

pub(super) struct TupleVariantSerializer {
    name: String,
    result: Vec<BencodeValue>,
}

impl ser::SerializeTupleVariant for TupleVariantSerializer {
    type Ok = BencodeValue;
    type Error = Error;

    fn serialize_field<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: serde::Serialize,
    {
        self.result.push(value.serialize(Serializer)?);
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        let mut dict = BTreeMap::new();
        dict.insert(
            self.name,
            BencodeValue::List(self.result.into_boxed_slice()),
        );
        Ok(BencodeValue::Dict(dict))
    }
}

pub(super) struct MapSerializer {
    result: BTreeMap<String, BencodeValue>,
    next_key: Option<String>,
}

struct MapKeySerializer;

impl ser::Serializer for MapKeySerializer {
    type Ok = String;
    type Error = Error;

    type SerializeSeq = ser::Impossible<String, Error>;
    type SerializeTuple = ser::Impossible<String, Error>;
    type SerializeTupleStruct = ser::Impossible<String, Error>;
    type SerializeTupleVariant = ser::Impossible<String, Error>;
    type SerializeMap = ser::Impossible<String, Error>;
    type SerializeStruct = ser::Impossible<String, Error>;
    type SerializeStructVariant = ser::Impossible<String, Error>;

    fn serialize_bool(self, v: bool) -> Result<Self::Ok, Self::Error> {
        Ok(v.to_string())
    }

    fn serialize_i8(self, v: i8) -> Result<Self::Ok, Self::Error> {
        Ok(v.to_string())
    }

    fn serialize_i16(self, v: i16) -> Result<Self::Ok, Self::Error> {
        Ok(v.to_string())
    }

    fn serialize_i32(self, v: i32) -> Result<Self::Ok, Self::Error> {
        Ok(v.to_string())
    }

    fn serialize_i64(self, v: i64) -> Result<Self::Ok, Self::Error> {
        Ok(v.to_string())
    }

    fn serialize_u8(self, v: u8) -> Result<Self::Ok, Self::Error> {
        Ok(v.to_string())
    }

    fn serialize_u16(self, v: u16) -> Result<Self::Ok, Self::Error> {
        Ok(v.to_string())
    }

    fn serialize_u32(self, v: u32) -> Result<Self::Ok, Self::Error> {
        Ok(v.to_string())
    }

    fn serialize_u64(self, v: u64) -> Result<Self::Ok, Self::Error> {
        Ok(v.to_string())
    }

    fn serialize_f32(self, _v: f32) -> Result<Self::Ok, Self::Error> {
        Err(anyhow!("invalid type: f32 not serializable to string").into())
    }

    fn serialize_f64(self, _v: f64) -> Result<Self::Ok, Self::Error> {
        Err(anyhow!("invalid type: f64 not serializable to string").into())
    }

    fn serialize_char(self, v: char) -> Result<Self::Ok, Self::Error> {
        Ok(v.to_string())
    }

    fn serialize_str(self, v: &str) -> Result<Self::Ok, Self::Error> {
        Ok(v.to_string())
    }

    fn serialize_bytes(self, v: &[u8]) -> Result<Self::Ok, Self::Error> {
        match std::str::from_utf8(v).ok() {
            Some(v) => Ok(v.to_string()),
            None => Err(anyhow!("map bytes key are not valid utf-8").into()),
        }
    }

    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        Err(anyhow!("invalid value: option none not serializable as string").into())
    }

    fn serialize_some<T: ?Sized>(self, value: &T) -> Result<Self::Ok, Self::Error>
    where
        T: serde::Serialize,
    {
        value.serialize(self)
    }

    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
        Err(anyhow!("invalid type: unit not serializable as string").into())
    }

    fn serialize_unit_struct(self, _name: &'static str) -> Result<Self::Ok, Self::Error> {
        Err(anyhow!("invalid type: unit struct not serializable as string").into())
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
    ) -> Result<Self::Ok, Self::Error> {
        Ok(variant.to_string())
    }

    fn serialize_newtype_struct<T: ?Sized>(
        self,
        _name: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: serde::Serialize,
    {
        value.serialize(self)
    }

    fn serialize_newtype_variant<T: ?Sized>(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: serde::Serialize,
    {
        Err(anyhow!("invalid type: newtype variant not serializable to string").into())
    }

    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        Err(anyhow!("invalid type: sequence not serializable to string").into())
    }

    fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple, Self::Error> {
        Err(anyhow!("invalid type: tuple not serializable to string").into())
    }

    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        Err(anyhow!("invalid type: tuple struct not serializable to string").into())
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        Err(anyhow!("invalid type: tuple variant not serializable to string").into())
    }

    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        Err(anyhow!("invalid type: map not serializable to string").into())
    }

    fn serialize_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        Err(anyhow!("invalid type: struct not serializable to string").into())
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        Err(anyhow!("invalid type: struct variant not serializable to string").into())
    }
}

impl ser::SerializeMap for MapSerializer {
    type Ok = BencodeValue;
    type Error = Error;

    fn serialize_key<T: ?Sized>(&mut self, key: &T) -> Result<(), Self::Error>
    where
        T: serde::Serialize,
    {
        self.next_key.replace(key.serialize(MapKeySerializer)?);
        Ok(())
    }

    fn serialize_value<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: serde::Serialize,
    {
        self.result.insert(
            self.next_key
                .take()
                .expect("serialize_value called before serialize_key"),
            value.serialize(Serializer)?,
        );
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        if self.next_key.is_some() {
            panic!(
                "serialize_key not matched with serialize_value before ending map serialization"
            );
        }
        Ok(BencodeValue::Dict(self.result))
    }
}

impl ser::SerializeStruct for MapSerializer {
    type Ok = BencodeValue;
    type Error = Error;

    fn serialize_field<T: ?Sized>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), Self::Error>
    where
        T: serde::Serialize,
    {
        self.result.insert(
            key.serialize(MapKeySerializer)?,
            value.serialize(Serializer)?,
        );
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(BencodeValue::Dict(self.result))
    }
}

pub(super) struct StructVariantSerializer {
    name: String,
    result: BTreeMap<String, BencodeValue>,
}

impl ser::SerializeStructVariant for StructVariantSerializer {
    type Ok = BencodeValue;
    type Error = Error;

    fn serialize_field<T: ?Sized>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), Self::Error>
    where
        T: serde::Serialize,
    {
        self.result.insert(
            key.serialize(MapKeySerializer)?,
            value.serialize(Serializer)?,
        );
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        let mut dict = BTreeMap::new();
        dict.insert(self.name, BencodeValue::Dict(self.result));
        Ok(BencodeValue::Dict(dict))
    }
}
