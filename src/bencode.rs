use std::collections::BTreeMap;

use anyhow::Result;
use bstr::BString;
use serde::{
    ser::{SerializeMap, SerializeSeq},
    Serialize,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BencodeValue {
    String(BString),
    Integer(i64),
    List(Box<[BencodeValue]>),
    Dict(BTreeMap<BString, BencodeValue>),
}

impl BencodeValue {
    /// Attempts to parse the bytes into a [`BencodeValue`].
    pub fn try_from_bytes(bytes: &[u8]) -> Result<Self> {
        Ok(bencode_parser::value(bytes)?)
    }
}

impl Serialize for BencodeValue {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            BencodeValue::String(value) => value.serialize(serializer),
            BencodeValue::Integer(n) => serializer.serialize_i64(*n),
            BencodeValue::List(l) => {
                let mut s = serializer.serialize_seq(Some(l.len()))?;
                for e in l.iter() {
                    s.serialize_element(e)?;
                }
                s.end()
            }
            BencodeValue::Dict(d) => {
                let mut s = serializer.serialize_map(Some(d.len()))?;
                for (k, v) in d.iter() {
                    s.serialize_entry(k, v)?;
                }
                s.end()
            }
        }
    }
}

peg::parser! {
    grammar bencode_parser() for [u8] {
        pub rule value() -> BencodeValue
            = s:bstring() { BencodeValue::String(s) }
            / n:binteger() { BencodeValue::Integer(n) }
            / l:blist() { BencodeValue::List(l) }
            / d:bdict() { BencodeValue::Dict(d) }

        /// Binary encoded string (`n:<some-content>`).
        rule bstring() -> BString = n:integer() ":" value:$([_]*<{n as usize}>) { BString::from(value) }
        /// Binary encoded integer (`d:<some-whole-number>e`).
        rule binteger() -> i64 = "i" sign:[b'-']? n:integer() "e" { sign.map(|_| -(n as i64)).unwrap_or(n as i64)}
        /// Binary encoded list of bencode values (`l<values-without-separators>e`).
        rule blist() -> Box<[BencodeValue]> = "l" l:value()* "e" { Box::from(l) }
        /// Binary encoded dictionary (`d<key-value-pairs>e`)
        rule bdict() -> BTreeMap<BString, BencodeValue> = "d" kvs:(k:bstring() v:value() { (k, v) })* "e" {
            BTreeMap::from_iter(kvs)
        }

        /// Unsigned natural number.
        rule integer() -> u64 = n:$((non_zero_digit() digit()*) / digit()) {?
            match std::str::from_utf8(n).map(|n| n.parse()) {
                Ok(Ok(n)) => Ok(n),
                _ => Err("unsigned 64 bit integer")
            }
        }

        rule non_zero_digit() -> u8 = quiet! { [c if c.is_ascii_digit() && c != b'0'] } / expected!("non-zero ascii digit")
        rule digit() -> u8 = quiet! { [c if c.is_ascii_digit()] } / expected!("ascii digit")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bstring() {
        let value0 = BencodeValue::try_from_bytes(b"6:foobar").unwrap();
        let value1 = BencodeValue::try_from_bytes(b"3:foo").unwrap();
        let value2 = BencodeValue::try_from_bytes(b"1:f").unwrap();
        let value3 = BencodeValue::try_from_bytes(b"15:foobarfoobarfoo").unwrap();
        let value4 = BencodeValue::try_from_bytes(b"0:").unwrap();

        assert_eq!(value0, BencodeValue::String("foobar".into()));
        assert_eq!(value1, BencodeValue::String("foo".into()));
        assert_eq!(value2, BencodeValue::String("f".into()));
        assert_eq!(value3, BencodeValue::String("foobarfoobarfoo".into()));
        assert_eq!(value4, BencodeValue::String("".into()));
    }

    #[test]
    fn binteger() {
        let value0 = BencodeValue::try_from_bytes(b"i42e").unwrap();
        let value1 = BencodeValue::try_from_bytes(b"i2147483647e").unwrap();
        let value2 = BencodeValue::try_from_bytes(b"i0e").unwrap();
        let value3 = BencodeValue::try_from_bytes(b"i-61e").unwrap();
        let value4 = BencodeValue::try_from_bytes(b"i4294967300e").unwrap();

        assert_eq!(value0, BencodeValue::Integer(42));
        assert_eq!(value1, BencodeValue::Integer(2147483647));
        assert_eq!(value2, BencodeValue::Integer(0));
        assert_eq!(value3, BencodeValue::Integer(-61));
        assert_eq!(value4, BencodeValue::Integer(4294967300));
    }

    #[test]
    fn blist() {
        let value0 = BencodeValue::try_from_bytes(b"l4:spami42ee").unwrap();
        let value1 = BencodeValue::try_from_bytes(b"l4:spame").unwrap();
        let value2 = BencodeValue::try_from_bytes(b"li42ee").unwrap();
        let value3 = BencodeValue::try_from_bytes(b"le").unwrap();
        let value4 = BencodeValue::try_from_bytes(b"lllleeee").unwrap();
        let value5 = BencodeValue::try_from_bytes(b"lli-42elleeee").unwrap();

        assert_eq!(
            value0,
            BencodeValue::List(Box::from([
                BencodeValue::String("spam".into()),
                BencodeValue::Integer(42)
            ]))
        );
        assert_eq!(
            value1,
            BencodeValue::List(Box::from([BencodeValue::String("spam".into()),]))
        );
        assert_eq!(
            value2,
            BencodeValue::List(Box::from([BencodeValue::Integer(42),]))
        );
        assert_eq!(value3, BencodeValue::List(Box::from([])));
        assert_eq!(
            value4,
            BencodeValue::List(Box::from([BencodeValue::List(Box::from([
                BencodeValue::List(Box::from([BencodeValue::List(Box::from([]))]))
            ]))]))
        );
        assert_eq!(
            value5,
            BencodeValue::List(Box::from([BencodeValue::List(Box::from([
                BencodeValue::Integer(-42),
                BencodeValue::List(Box::from([BencodeValue::List(Box::from([]))]))
            ]))]))
        );
    }

    #[test]
    fn bdict() {
        let value0 = BencodeValue::try_from_bytes(b"d4:spam3:fooe").unwrap();
        let value1 = BencodeValue::try_from_bytes(b"d1:ei-40012ee").unwrap();
        let value2 = BencodeValue::try_from_bytes(b"de").unwrap();
        let value3 = BencodeValue::try_from_bytes(b"d4:spam3:foo1:bi8ee").unwrap();
        let value4 =
            BencodeValue::try_from_bytes(b"d4:spamd4:spamd4:spamd4:spamd4:spami-42eeeeee").unwrap();
        let value5 = BencodeValue::try_from_bytes(b"d1:el3:bard1:ei-1008eeee").unwrap();

        assert_eq!(
            value0,
            BencodeValue::Dict(BTreeMap::from([(
                "spam".into(),
                BencodeValue::String("foo".into())
            )]))
        );
        assert_eq!(
            value1,
            BencodeValue::Dict(BTreeMap::from([(
                "e".into(),
                BencodeValue::Integer(-40012)
            )]))
        );
        assert_eq!(value2, BencodeValue::Dict(BTreeMap::from([])));
        assert_eq!(
            value3,
            BencodeValue::Dict(BTreeMap::from([
                ("spam".into(), BencodeValue::String("foo".into())),
                ("b".into(), BencodeValue::Integer(8))
            ]))
        );
        assert_eq!(
            value4,
            BencodeValue::Dict(BTreeMap::from([(
                "spam".into(),
                BencodeValue::Dict(BTreeMap::from([(
                    "spam".into(),
                    BencodeValue::Dict(BTreeMap::from([(
                        "spam".into(),
                        BencodeValue::Dict(BTreeMap::from([(
                            "spam".into(),
                            BencodeValue::Dict(BTreeMap::from([(
                                "spam".into(),
                                BencodeValue::Integer(-42)
                            )]))
                        )]))
                    )]))
                )]))
            ),]))
        );
        assert_eq!(
            value5,
            BencodeValue::Dict(BTreeMap::from([(
                "e".into(),
                BencodeValue::List(Box::from([
                    BencodeValue::String("bar".into()),
                    BencodeValue::Dict(BTreeMap::from([(
                        "e".into(),
                        BencodeValue::Integer(-1008)
                    )]))
                ]))
            ),]))
        );
    }
}
