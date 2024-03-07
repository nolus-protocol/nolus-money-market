use std::{
    fs::File,
    io::{self, BufRead as _, BufReader},
    path::Path,
};

use cosmwasm_std::Storage;
use data_encoding::DecodeError;

use self::kv_pair::KvPair;

pub mod kv_pair;

/// CSV format should be as follows:
/// ```plaintext
/// KEY,VALUE
/// KEY,VALUE
/// ...
/// KEY,VALUE
/// ```
/// Whitespaces in front, between and in the end shall be ignored.
///
/// `KEY` represents the key-value pair's key and has to be encoded in uppercase
/// hexadecimal encoding.
///
/// `VALUE` represents the key-value pair's value and has to be encoded in
/// padded uppercase BASE64 encoding.
///
/// Simplified version of the shell script required to export contract's state
/// is as follows:
/// ```sh
/// nolusd q wasm cs all '{{{ CONTRACT }}}' --output json \
///     | jq '.models[] | .key + "," + .value' \
///     >> exported.csv
/// ```
pub fn try_load_into_storage_from_csv<S>(
    storage: &mut S,
    path: &Path,
) -> Result<(), LoadIntoStorageFromFileError>
where
    S: Storage + ?Sized,
{
    File::open(path)
        .map(BufReader::new)
        .map_err(LoadIntoStorageFromFileError::Io)
        .and_then(|iter| try_load_into_storage(storage, parse_csv_lines(iter)))
}

#[derive(Debug, thiserror::Error)]
pub enum LoadIntoStorageFromFileError {
    #[error("I/O operation error has occurred! Cause: {0}")]
    Io(io::Error),
    #[error("Delimiter not found!")]
    DelimiterNotFound,
    #[error("Decoding error has occurred! Cause: {0}")]
    Decode(DecodeError),
}

fn try_load_into_storage<S, I, E>(storage: &mut S, mut iter: I) -> Result<(), E>
where
    S: Storage + ?Sized,
    I: Iterator<Item = Result<KvPair, E>>,
{
    iter.try_for_each(|kv_pair| {
        kv_pair.map(|kv_pair| storage.set(kv_pair.key().as_ref(), kv_pair.value().as_ref()))
    })
}

fn parse_csv_lines(
    iter: BufReader<File>,
) -> impl Iterator<Item = Result<KvPair, LoadIntoStorageFromFileError>> {
    iter.lines().map(|result| {
        result
            .map_err(LoadIntoStorageFromFileError::Io)
            .and_then(parse_csv_line)
    })
}

fn parse_csv_line(line: String) -> Result<KvPair, LoadIntoStorageFromFileError> {
    line.split_once(',')
        .map(|(key, value)| (key.trim().as_bytes(), value.trim().as_bytes()))
        .ok_or(LoadIntoStorageFromFileError::DelimiterNotFound)
        .and_then(|(key, value)| {
            KvPair::try_from_encoded(key, value).map_err(LoadIntoStorageFromFileError::Decode)
        })
}
