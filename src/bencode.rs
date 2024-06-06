use anyhow::Result;
use bstr::BString;
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BencodeValue {
    BString(BString),
    BInteger(i64),
}

impl BencodeValue {
    /// Tries to parse the bytes into a [`BencodeValue`].
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
            BencodeValue::BString(value) => value.serialize(serializer),
            BencodeValue::BInteger(n) => serializer.serialize_i64(*n),
        }
    }
}

peg::parser! {
    grammar bencode_parser() for [u8] {
        pub rule value() -> BencodeValue
            = s:bstring() { BencodeValue::BString(s) }
            / n:binteger() { BencodeValue::BInteger(n) }

        /// Binary encoded string (`n:<some-content>`).
        rule bstring() -> BString = n:integer() ":" value:$([_]*<{n as usize}>) { BString::from(value) }
        /// Binary encoded integer (`d:<some-whole-number>e`).
        rule binteger() -> i64 = "i" sign:[b'-']? n:integer() "e" { sign.map(|_| -(n as i64)).unwrap_or(n as i64)}

        /// Unsigned natural number.
        rule integer() -> u64 = n:$((non_zero_digit() digit()*) / digit()) {?
            match std::str::from_utf8(n).map(|n| n.parse()) {
                Ok(Ok(n)) => Ok(n),
                _ => Err("unsigned 64 bit integer")
            }
        }

        rule non_zero_digit() -> u8 = [c if c.is_ascii_digit() && c != b'0']
        rule digit() -> u8 = [c if c.is_ascii_digit()]
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

        assert_eq!(value0, BencodeValue::BString("foobar".into()));
        assert_eq!(value1, BencodeValue::BString("foo".into()));
        assert_eq!(value2, BencodeValue::BString("f".into()));
        assert_eq!(value3, BencodeValue::BString("foobarfoobarfoo".into()));
        assert_eq!(value4, BencodeValue::BString("".into()));
    }

    #[test]
    fn binteger() {
        let value0 = BencodeValue::try_from_bytes(b"i42e").unwrap();
        let value1 = BencodeValue::try_from_bytes(b"i2147483647e").unwrap();
        let value2 = BencodeValue::try_from_bytes(b"i0e").unwrap();
        let value3 = BencodeValue::try_from_bytes(b"i-61e").unwrap();
        let value4 = BencodeValue::try_from_bytes(b"i4294967300e").unwrap();

        assert_eq!(value0, BencodeValue::BInteger(42));
        assert_eq!(value1, BencodeValue::BInteger(2147483647));
        assert_eq!(value2, BencodeValue::BInteger(0));
        assert_eq!(value3, BencodeValue::BInteger(-61));
        assert_eq!(value4, BencodeValue::BInteger(4294967300));
    }
}
