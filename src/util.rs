pub mod serde_with {
    use std::marker::PhantomData;

    use bytes::{Bytes, BytesMut};
    use serde::{de, Deserialize};
    use serde_with::{DeserializeAs, SerializeAs};

    /// Parses bytes as chunks with specified length.
    pub struct ByteChunksWithLength<const N: usize>(PhantomData<[u8; N]>);

    impl<'de, const N: usize> DeserializeAs<'de, Vec<Bytes>> for ByteChunksWithLength<N> {
        fn deserialize_as<D>(deserializer: D) -> Result<Vec<Bytes>, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            let s = Bytes::deserialize(deserializer).map_err(de::Error::custom)?;
            Ok(s.chunks(N).map(Bytes::copy_from_slice).collect())
        }
    }

    impl<const N: usize, I> SerializeAs<I> for ByteChunksWithLength<N>
    where
        for<'a> &'a I: IntoIterator<Item = &'a Bytes>,
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
