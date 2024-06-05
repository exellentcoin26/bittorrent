use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum BencodeValue {
    BString(String),
}

impl std::str::FromStr for BencodeValue {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(bencode_parser::value(s)?)
    }
}

peg::parser! {
    grammar bencode_parser() for str {
        pub rule value() -> BencodeValue = s:bstring() { BencodeValue::BString(s) }

        /// Binary string encoded as `n:<some-content>`
        rule bstring() -> String = n:integer() ":" value:$([_]*<{n as usize}>) { value.to_string() }

        /// Unsigned natural number.
        rule integer() -> u32 = n:$((non_zero_digit() digit()*) / digit()) {? n.parse().or(Err("non zero length"))}

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
}
