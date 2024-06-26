pub type PeerId = [u8; 20];
pub type InfoHash = [u8; 20];
pub type PieceHash = [u8; 20];

pub mod serde_with {
    use std::marker::PhantomData;

    use bytes::{Bytes, BytesMut};
    use serde::{de, Deserialize};
    use serde_with::{DeserializeAs, SerializeAs};

    /// Parses bytes as chunks with specified length.
    pub struct ArrayChunksWithLength<const N: usize>(PhantomData<[u8; N]>);

    impl<'de, const N: usize> DeserializeAs<'de, Vec<[u8; N]>> for ArrayChunksWithLength<N> {
        fn deserialize_as<D>(deserializer: D) -> Result<Vec<[u8; N]>, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            let s = Bytes::deserialize(deserializer).map_err(de::Error::custom)?;
            let mut chunks = s.chunks_exact(N);
            let result = chunks
                .by_ref()
                .map(|c| c.try_into().expect("chunk size should be N exactly"))
                .collect();

            if !chunks.remainder().is_empty() {
                return Err(de::Error::custom("byte size should be divisible by N"));
            }

            Ok(result)
        }
    }

    impl<const N: usize, I> SerializeAs<I> for ArrayChunksWithLength<N>
    where
        for<'a> &'a I: IntoIterator<Item = &'a [u8; N]>,
    {
        fn serialize_as<S>(source: &I, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
        {
            serializer.serialize_bytes(&BytesMut::from_iter(
                source.into_iter().flat_map(|b| b.as_ref().iter()),
            ))
        }
    }
}
