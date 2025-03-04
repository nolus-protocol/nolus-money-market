use crate::cosmwasm_std::HexBinary;

use super::LoadIntoStorageFromFileError;

pub struct KvPair {
    key: Vec<u8>,
    value: Vec<u8>,
}

impl KvPair {
    pub fn try_from_encoded(
        key_encoded: &str,
        value_encoded: &str,
    ) -> Result<Self, LoadIntoStorageFromFileError> {
        HexBinary::from_hex(key_encoded)
            .map_err(LoadIntoStorageFromFileError::DecodeKey)
            .and_then(|key| {
                use base64::{Engine, engine::general_purpose};
                general_purpose::STANDARD
                    .decode(value_encoded)
                    .map_err(LoadIntoStorageFromFileError::DecodeValue)
                    .map(|value| Self {
                        key: key.into(),
                        value,
                    })
            })
    }

    pub fn key(&self) -> &[u8] {
        &self.key
    }

    pub fn value(&self) -> &[u8] {
        &self.value
    }
}
