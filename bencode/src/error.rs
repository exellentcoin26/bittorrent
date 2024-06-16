use derive_more::{Display, From};
use serde::{de, ser};

#[derive(Debug, Display, From)]
pub enum Error {
    #[from]
    Generic(anyhow::Error),
}

impl std::error::Error for Error {}

impl ser::Error for Error {
    fn custom<T>(msg: T) -> Self
    where
        T: std::fmt::Display,
    {
        Self::from(anyhow::Error::msg(msg.to_string()))
    }
}

impl de::Error for Error {
    fn custom<T>(msg: T) -> Self
    where
        T: std::fmt::Display,
    {
        Self::from(anyhow::Error::msg(msg.to_string()))
    }
}
