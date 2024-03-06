use data_encoding::DecodeError;

pub struct KvPair {
    key: Box<[u8]>,
    value: Box<[u8]>,
}

impl KvPair {
    pub const fn from_raw(key: Box<[u8]>, value: Box<[u8]>) -> Self {
        Self { key, value }
    }

    pub fn try_from_encoded(key: &[u8], value: &[u8]) -> Result<Self, DecodeError> {
        data_encoding::HEXUPPER.decode(key).and_then(|key| {
            data_encoding::BASE64
                .decode(value)
                .map(|value| Self::from_raw(key.into_boxed_slice(), value.into_boxed_slice()))
        })
    }

    pub fn key(&self) -> &[u8] {
        &self.key
    }

    pub fn value(&self) -> &[u8] {
        &self.value
    }
}
