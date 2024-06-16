use std::collections::BTreeMap;

use anyhow::{Context, Result};
use bstr::BString;
use format_bytes::write_bytes;
use serde::{
    ser::{SerializeMap, SerializeSeq},
    Deserialize, Serialize,
};

use self::ser::Serializer;

mod de;
mod error;
mod ser;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BencodeValue {
    String(BString),
    Integer(i64),
    List(Box<[BencodeValue]>),
    Dict(BTreeMap<String, BencodeValue>),
}

impl BencodeValue {
    /// Attempts to parse the bytes into a [`BencodeValue`].
    pub fn try_from_bytes(bytes: &[u8]) -> Result<Self> {
        Ok(bencode_parser::value(bytes)?)
    }

    pub fn to_byte_string(&self) -> std::io::Result<BString> {
        use std::io::Write;

        let mut buf = Vec::new();
        match self {
            BencodeValue::String(s) => write_bytes!(&mut buf, b"{}:{}", s.len(), **s)?,
            BencodeValue::Integer(i) => write_bytes!(&mut buf, b"i{}e", i)?,
            BencodeValue::List(l) => {
                write!(&mut buf, "l")?;
                for v in l.iter() {
                    let v = v.to_byte_string()?;
                    write_bytes!(&mut buf, b"{}", *v)?;
                }
                write!(&mut buf, "e")?;
            }
            BencodeValue::Dict(d) => {
                write!(&mut buf, "d")?;
                for (k, v) in d.iter() {
                    let v = v.to_byte_string()?;
                    write_bytes!(&mut buf, b"{}:{}{}", k.len(), k.as_bytes(), *v)?;
                }
                write!(&mut buf, "e")?;
            }
        };
        Ok(BString::new(buf))
    }

    pub fn from_serialize<T: Serialize>(value: T) -> Result<Self> {
        value
            .serialize(Serializer)
            .context("failed to serialize value to bencode")
    }

    pub fn into_deserialize<T: for<'de> Deserialize<'de>>(self) -> Result<T> {
        T::deserialize(self).context("failed to deserialize bencode value into requested type")
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
        rule bdict() -> BTreeMap<String, BencodeValue> = "d" kvs:(k:bstring() v:value() { (k.to_string(), v) })* "e" {
            BTreeMap::from_iter(kvs)
        }

        /// Unsigned natural number.
        rule integer() -> u64 = n:$((non_zero_digit() digit()*) / digit()) {?
            std::str::from_utf8(n).map_err(|_| ())
                .and_then(|n| n.parse().map_err(|_| ()))
                .or(Err("unsigned 64 bit integer"))
        }

        rule non_zero_digit() -> u8 = quiet! { [c if c.is_ascii_digit() && c != b'0'] } / expected!("non-zero ascii digit")
        rule digit() -> u8 = quiet! { [c if c.is_ascii_digit()] } / expected!("ascii digit")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod parse {
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
                BencodeValue::try_from_bytes(b"d4:spamd4:spamd4:spamd4:spamd4:spami-42eeeeee")
                    .unwrap();
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

    mod to_byte_string {
        use super::*;
        use bstr::B;

        #[test]
        fn bstring() {
            let value0 = BencodeValue::String("foobar".into());
            let value1 = BencodeValue::String("foo".into());
            let value2 = BencodeValue::String("f".into());
            let value3 = BencodeValue::String("foobarfoobarfoo".into());
            let value4 = BencodeValue::String("".into());

            assert_eq!(value0.to_byte_string().unwrap(), B(b"6:foobar"));
            assert_eq!(value1.to_byte_string().unwrap(), B(b"3:foo"));
            assert_eq!(value2.to_byte_string().unwrap(), B(b"1:f"));
            assert_eq!(value3.to_byte_string().unwrap(), B(b"15:foobarfoobarfoo"));
            assert_eq!(value4.to_byte_string().unwrap(), B(b"0:"));
        }

        #[test]
        fn binteger() {
            let value0 = BencodeValue::Integer(42);
            let value1 = BencodeValue::Integer(2147483647);
            let value2 = BencodeValue::Integer(0);
            let value3 = BencodeValue::Integer(-61);
            let value4 = BencodeValue::Integer(4294967300);

            assert_eq!(value0.to_byte_string().unwrap(), B(b"i42e"));
            assert_eq!(value1.to_byte_string().unwrap(), B(b"i2147483647e"));
            assert_eq!(value2.to_byte_string().unwrap(), B(b"i0e"));
            assert_eq!(value3.to_byte_string().unwrap(), B(b"i-61e"));
            assert_eq!(value4.to_byte_string().unwrap(), B(b"i4294967300e"));
        }

        #[test]
        fn blist() {
            let value0 = BencodeValue::List(Box::from([
                BencodeValue::String("spam".into()),
                BencodeValue::Integer(42),
            ]));
            let value1 = BencodeValue::List(Box::from([BencodeValue::String("spam".into())]));
            let value2 = BencodeValue::List(Box::from([BencodeValue::Integer(42)]));
            let value3 = BencodeValue::List(Box::from([]));
            let value4 = BencodeValue::List(Box::from([BencodeValue::List(Box::from([
                BencodeValue::List(Box::from([BencodeValue::List(Box::from([]))])),
            ]))]));
            let value5 = BencodeValue::List(Box::from([BencodeValue::List(Box::from([
                BencodeValue::Integer(-42),
                BencodeValue::List(Box::from([BencodeValue::List(Box::from([]))])),
            ]))]));

            assert_eq!(value0.to_byte_string().unwrap(), B(b"l4:spami42ee"));
            assert_eq!(value1.to_byte_string().unwrap(), B(b"l4:spame"));
            assert_eq!(value2.to_byte_string().unwrap(), B(b"li42ee"));
            assert_eq!(value3.to_byte_string().unwrap(), B(b"le"));
            assert_eq!(value4.to_byte_string().unwrap(), B(b"lllleeee"));
            assert_eq!(value5.to_byte_string().unwrap(), B(b"lli-42elleeee"));
        }

        #[test]
        fn bdict() {
            let value0 = BencodeValue::Dict(BTreeMap::from([(
                "spam".into(),
                BencodeValue::String("foo".into()),
            )]));
            let value1 = BencodeValue::Dict(BTreeMap::from([(
                "e".into(),
                BencodeValue::Integer(-40012),
            )]));
            let value2 = BencodeValue::Dict(BTreeMap::from([]));
            let value3 = BencodeValue::Dict(BTreeMap::from([
                ("spam".into(), BencodeValue::String("foo".into())),
                ("b".into(), BencodeValue::Integer(8)),
            ]));
            let value4 = BencodeValue::Dict(BTreeMap::from([(
                "spam".into(),
                BencodeValue::Dict(BTreeMap::from([(
                    "spam".into(),
                    BencodeValue::Dict(BTreeMap::from([(
                        "spam".into(),
                        BencodeValue::Dict(BTreeMap::from([(
                            "spam".into(),
                            BencodeValue::Dict(BTreeMap::from([(
                                "spam".into(),
                                BencodeValue::Integer(-42),
                            )])),
                        )])),
                    )])),
                )])),
            )]));
            let value5 = BencodeValue::Dict(BTreeMap::from([(
                "e".into(),
                BencodeValue::List(Box::from([
                    BencodeValue::String("bar".into()),
                    BencodeValue::Dict(BTreeMap::from([(
                        "e".into(),
                        BencodeValue::Integer(-1008),
                    )])),
                ])),
            )]));

            assert_eq!(value0.to_byte_string().unwrap(), B(b"d4:spam3:fooe"));
            assert_eq!(value1.to_byte_string().unwrap(), B(b"d1:ei-40012ee"));
            assert_eq!(value2.to_byte_string().unwrap(), B(b"de"));
            assert_eq!(value3.to_byte_string().unwrap(), B(b"d1:bi8e4:spam3:fooe"));
            assert_eq!(
                value4.to_byte_string().unwrap(),
                B(b"d4:spamd4:spamd4:spamd4:spamd4:spami-42eeeeee")
            );
            assert_eq!(
                value5.to_byte_string().unwrap(),
                B(b"d1:el3:bard1:ei-1008eeee")
            );
        }
    }
}
