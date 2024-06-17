use std::collections::BTreeMap;

use bstr::BString;
use serde::de::{self, value::MapDeserializer, Error as DeError, IntoDeserializer};

use super::{error::Error, BencodeValue};

impl<'de> de::Deserialize<'de> for BencodeValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct BencodeValueVisitor;

        impl<'de> de::Visitor<'de> for BencodeValueVisitor {
            type Value = BencodeValue;

            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                write!(f, "any valid bencode value")
            }

            fn visit_bool<E>(self, v: bool) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                self.visit_i64(i64::from(v))
            }

            fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(BencodeValue::Integer(v))
            }

            fn visit_i128<E>(self, v: i128) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                match i64::try_from(v).ok() {
                    Some(v) => self.visit_i64(v),
                    None => Err(de::Error::invalid_value(
                        de::Unexpected::Other("i128 not storable in an i64"),
                        &self,
                    )),
                }
            }

            fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                match i64::try_from(v).ok() {
                    Some(v) => self.visit_i64(v),
                    None => Err(de::Error::invalid_value(
                        de::Unexpected::Other("u64 not storable in an i64"),
                        &self,
                    )),
                }
            }

            fn visit_u128<E>(self, v: u128) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                match i64::try_from(v).ok() {
                    Some(v) => self.visit_i64(v),
                    None => Err(de::Error::invalid_value(
                        de::Unexpected::Other("u128 not storable in an i64"),
                        &self,
                    )),
                }
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                self.visit_bytes(v.as_bytes())
            }

            fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                self.visit_byte_buf(v.to_vec())
            }

            fn visit_byte_buf<E>(self, v: Vec<u8>) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(BencodeValue::String(BString::new(v)))
            }

            fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                de::Deserialize::deserialize(deserializer)
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: de::SeqAccess<'de>,
            {
                let mut vec = Vec::new();
                while let Some(elem) = seq.next_element()? {
                    vec.push(elem);
                }
                Ok(BencodeValue::List(vec.into_boxed_slice()))
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: de::MapAccess<'de>,
            {
                let mut dict = BTreeMap::new();
                while let Some((key, value)) = map.next_entry()? {
                    dict.insert(key, value);
                }
                Ok(BencodeValue::Dict(dict))
            }
        }

        deserializer.deserialize_any(BencodeValueVisitor)
    }
}

impl<'de> de::Deserializer<'de> for BencodeValue {
    type Error = Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        match self {
            BencodeValue::String(s) => visitor.visit_bytes(&s),
            BencodeValue::Integer(i) => visitor.visit_i64(i),
            BencodeValue::List(l) => visitor.visit_seq(l.to_vec().into_deserializer()),
            BencodeValue::Dict(d) => visitor.visit_map(d.into_deserializer()),
        }
    }

    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        match self {
            BencodeValue::Integer(i) => visitor.visit_bool(i != 0),
            _ => Err(Error::invalid_type(self.unexpected(), &visitor)),
        }
    }

    fn deserialize_i8<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        let BencodeValue::Integer(i) = self else {
            return Err(Error::invalid_type(self.unexpected(), &visitor));
        };
        match i8::try_from(i).ok() {
            Some(i) => visitor.visit_i8(i),
            None => Err(Error::invalid_value(self.unexpected(), &visitor)),
        }
    }

    fn deserialize_i16<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        let BencodeValue::Integer(i) = self else {
            return Err(Error::invalid_type(self.unexpected(), &visitor));
        };
        match i16::try_from(i).ok() {
            Some(i) => visitor.visit_i16(i),
            None => Err(Error::invalid_value(self.unexpected(), &visitor)),
        }
    }

    fn deserialize_i32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        let BencodeValue::Integer(i) = self else {
            return Err(Error::invalid_type(self.unexpected(), &visitor));
        };
        match i32::try_from(i).ok() {
            Some(i) => visitor.visit_i32(i),
            None => Err(Error::invalid_value(self.unexpected(), &visitor)),
        }
    }

    fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        match self {
            BencodeValue::Integer(i) => visitor.visit_i64(i),
            _ => Err(Error::invalid_type(self.unexpected(), &visitor)),
        }
    }

    fn deserialize_u8<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        let BencodeValue::Integer(i) = self else {
            return Err(Error::invalid_type(self.unexpected(), &visitor));
        };
        match u8::try_from(i).ok() {
            Some(i) => visitor.visit_u8(i),
            None => Err(Error::invalid_value(self.unexpected(), &visitor)),
        }
    }

    fn deserialize_u16<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        let BencodeValue::Integer(i) = self else {
            return Err(Error::invalid_type(self.unexpected(), &visitor));
        };
        match u16::try_from(i).ok() {
            Some(i) => visitor.visit_u16(i),
            None => Err(Error::invalid_value(self.unexpected(), &visitor)),
        }
    }

    fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        let BencodeValue::Integer(i) = self else {
            return Err(Error::invalid_type(self.unexpected(), &visitor));
        };
        match u32::try_from(i).ok() {
            Some(i) => visitor.visit_u32(i),
            None => Err(Error::invalid_value(self.unexpected(), &visitor)),
        }
    }

    fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        let BencodeValue::Integer(i) = self else {
            return Err(Error::invalid_type(self.unexpected(), &visitor));
        };
        match u64::try_from(i).ok() {
            Some(i) => visitor.visit_u64(i),
            None => Err(Error::invalid_value(self.unexpected(), &visitor)),
        }
    }

    fn deserialize_f32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        match self {
            BencodeValue::Integer(i) => visitor.visit_f32(i as f32),
            _ => Err(Error::invalid_type(self.unexpected(), &visitor)),
        }
    }

    fn deserialize_f64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        match self {
            BencodeValue::Integer(i) => visitor.visit_f64(i as f64),
            _ => Err(Error::invalid_type(self.unexpected(), &visitor)),
        }
    }

    fn deserialize_char<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        self.deserialize_string(visitor)
    }

    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        self.deserialize_string(visitor)
    }

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        self.deserialize_byte_buf(visitor)
    }

    fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        self.deserialize_byte_buf(visitor)
    }

    fn deserialize_byte_buf<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        match self {
            BencodeValue::String(s) => visitor.visit_bytes(s.as_slice()),
            BencodeValue::List(l) => visitor.visit_seq(l.to_vec().into_deserializer()),
            _ => Err(Error::invalid_type(self.unexpected(), &visitor)),
        }
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        visitor.visit_some(self)
    }

    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        Err(Error::invalid_type(self.unexpected(), &visitor))
    }

    fn deserialize_unit_struct<V>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        self.deserialize_unit(visitor)
    }

    fn deserialize_newtype_struct<V>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        visitor.visit_newtype_struct(self)
    }

    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        match self {
            BencodeValue::String(s) => visitor.visit_bytes(s.as_slice()),
            BencodeValue::List(l) => visitor.visit_seq(l.to_vec().into_deserializer()),
            _ => Err(Error::invalid_type(self.unexpected(), &visitor)),
        }
    }

    fn deserialize_tuple<V>(self, _len: usize, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        self.deserialize_seq(visitor)
    }

    fn deserialize_tuple_struct<V>(
        self,
        _name: &'static str,
        _len: usize,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        self.deserialize_seq(visitor)
    }

    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        let BencodeValue::Dict(d) = self else {
            return Err(Error::invalid_type(self.unexpected(), &visitor));
        };
        let mut d = d.into_deserializer();
        let result = visitor.visit_map(&mut d)?;
        d.end().map(|_| result)
    }

    fn deserialize_struct<V>(
        self,
        _name: &'static str,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        self.deserialize_map(visitor)
    }

    fn deserialize_enum<V>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        use bstr::ByteVec;

        let deserializer: EnumDeserializer = match self {
            BencodeValue::Dict(d) => {
                de::value::MapAccessDeserializer::new(d.into_deserializer()).into()
            }
            BencodeValue::String(s) => Vec::from(s)
                .into_string()
                .map_err(|e| {
                    Error::invalid_value(de::Unexpected::Bytes(e.as_bytes()), &"valid utf-8 string")
                })?
                .into_deserializer()
                .into(),
            other => {
                return Err(Error::invalid_value(
                    other.unexpected(),
                    &"map with a single key or a string",
                ))
            }
        };

        visitor.visit_enum(deserializer)
    }

    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        self.deserialize_string(visitor)
    }

    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        visitor.visit_unit()
    }
}

#[derive(derive_more::From)]
enum EnumDeserializer<'de> {
    String(de::value::StringDeserializer<Error>),
    Struct(
        de::value::MapAccessDeserializer<
            MapDeserializer<'de, <BTreeMap<String, BencodeValue> as IntoIterator>::IntoIter, Error>,
        >,
    ),
}

#[allow(clippy::type_complexity)]
enum EnumVariantKind<'de> {
    Unit(<de::value::StringDeserializer<Error> as de::EnumAccess<'de>>::Variant),
    Struct(
        <de::value::MapAccessDeserializer<
            MapDeserializer<'de, <BTreeMap<String, BencodeValue> as IntoIterator>::IntoIter, Error>,
        > as de::EnumAccess<'de>>::Variant,
    ),
}

impl<'de> de::EnumAccess<'de> for EnumDeserializer<'de> {
    type Error = Error;
    type Variant = EnumVariantKind<'de>;

    fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant), Self::Error>
    where
        V: de::DeserializeSeed<'de>,
    {
        match self {
            EnumDeserializer::String(s) => s
                .variant_seed(seed)
                .map(|(v, va)| (v, EnumVariantKind::Unit(va))),
            EnumDeserializer::Struct(s) => s
                .variant_seed(seed)
                .map(|(v, va)| (v, EnumVariantKind::Struct(va))),
        }
    }
}

impl<'de> de::VariantAccess<'de> for EnumVariantKind<'de> {
    type Error = Error;

    fn unit_variant(self) -> Result<(), Self::Error> {
        match self {
            EnumVariantKind::Unit(u) => u.unit_variant(),
            EnumVariantKind::Struct(s) => s.unit_variant(),
        }
    }

    fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value, Self::Error>
    where
        T: de::DeserializeSeed<'de>,
    {
        match self {
            EnumVariantKind::Unit(u) => u.newtype_variant_seed(seed),
            EnumVariantKind::Struct(s) => s.newtype_variant_seed(seed),
        }
    }

    fn tuple_variant<V>(self, len: usize, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        match self {
            EnumVariantKind::Unit(u) => u.tuple_variant(len, visitor),
            EnumVariantKind::Struct(s) => s.tuple_variant(len, visitor),
        }
    }

    fn struct_variant<V>(
        self,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        match self {
            EnumVariantKind::Unit(u) => u.struct_variant(fields, visitor),
            EnumVariantKind::Struct(s) => s.struct_variant(fields, visitor),
        }
    }
}

impl BencodeValue {
    fn unexpected(&self) -> de::Unexpected {
        match self {
            BencodeValue::String(s) => de::Unexpected::Bytes(s),
            BencodeValue::Integer(i) => de::Unexpected::Signed(*i),
            BencodeValue::List(_) => de::Unexpected::Seq,
            BencodeValue::Dict(_) => de::Unexpected::Map,
        }
    }
}

impl<'de> IntoDeserializer<'de, Error> for BencodeValue {
    type Deserializer = Self;

    fn into_deserializer(self) -> Self::Deserializer {
        self
    }
}
