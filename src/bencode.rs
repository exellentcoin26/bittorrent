use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BencodeValue {
    BString(String),
    BInteger(i64),
}

impl std::str::FromStr for BencodeValue {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(bencode_parser::value(s)?)
    }
}

impl Serialize for BencodeValue {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            BencodeValue::BString(value) => serializer.serialize_str(value),
            BencodeValue::BInteger(n) => serializer.serialize_i64(*n),
        }
    }
}

peg::parser! {
    grammar bencode_parser() for str {
        pub rule value() -> BencodeValue
            = s:bstring() { BencodeValue::BString(s) }
            / n:binteger() { BencodeValue::BInteger(n) }

        /// Binary encoded string (`n:<some-content>`).
        rule bstring() -> String = n:integer() ":" value:$([_]*<{n as usize}>) { value.to_string() }
        /// Binary encoded integer (`d:<some-whole-number>e`).
        rule binteger() -> i64 = "i" sign:['-']? n:integer() "e" { sign.map(|_| -(n as i64)).unwrap_or(n as i64)}

        /// Unsigned natural number.
        rule integer() -> u64 = n:$((non_zero_digit() digit()*) / digit()) {? n.parse().or(Err("non zero length"))}

        rule non_zero_digit() -> char = [c if c.is_ascii_digit() && c != '0']
        rule digit() -> char = [c if c.is_ascii_digit()]
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::*;

    #[test]
    fn bstring() {
        let value0 = BencodeValue::from_str("6:foobar").unwrap();
        let value1 = BencodeValue::from_str("3:foo").unwrap();
        let value2 = BencodeValue::from_str("1:f").unwrap();
        let value3 = BencodeValue::from_str("15:foobarfoobarfoo").unwrap();
        let value4 = BencodeValue::from_str("0:").unwrap();

        assert_eq!(value0, BencodeValue::BString("foobar".to_string()));
        assert_eq!(value1, BencodeValue::BString("foo".to_string()));
        assert_eq!(value2, BencodeValue::BString("f".to_string()));
        assert_eq!(value3, BencodeValue::BString("foobarfoobarfoo".to_string()));
        assert_eq!(value4, BencodeValue::BString("".to_string()));
    }

    #[test]
    fn binteger() {
        let value0 = BencodeValue::from_str("i42e").unwrap();
        let value1 = BencodeValue::from_str("i2147483647e").unwrap();
        let value2 = BencodeValue::from_str("i0e").unwrap();
        let value3 = BencodeValue::from_str("i-61e").unwrap();
        let value4 = BencodeValue::from_str("i4294967300e").unwrap();

        assert_eq!(value0, BencodeValue::BInteger(42));
        assert_eq!(value1, BencodeValue::BInteger(2147483647));
        assert_eq!(value2, BencodeValue::BInteger(0));
        assert_eq!(value3, BencodeValue::BInteger(-61));
        assert_eq!(value4, BencodeValue::BInteger(4294967300));
    }
}
