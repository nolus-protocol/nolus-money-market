use std::io::{Error as IOError, Write};

use serde::{de::Error as DeserializationError, Deserialize, Deserializer, Serialize, Serializer};

use crate::Error;

#[derive(Serialize, Deserialize)]
#[serde(try_from = "SerializedCurrency")]
pub(super) struct Currency {
    name: String,
    ticker: String,
    #[serde(skip)]
    normalized_ticker: String,
    symbol: String,
    #[serde(serialize_with = "serialize_decimal_digits")]
    decimal_digits: u8,
    path: String,
    groups: Vec<String>,
    resolution_paths: Vec<Vec<String>>,
}

impl TryFrom<SerializedCurrency> for Currency {
    type Error = Error;

    fn try_from(currency: SerializedCurrency) -> Result<Self, Self::Error> {
        let normalized_ticker = currency
            .ticker
            .get(..1)
            .ok_or_else(|| {
                Error::Deserialization("Currency ticker requires non-empty ticker!".into())
            })?
            .to_ascii_uppercase()
            + &currency.ticker[1..].to_ascii_lowercase();

        Ok(Self {
            name: currency.name,
            ticker: currency.ticker,
            normalized_ticker,
            symbol: currency.symbol,
            decimal_digits: currency.decimal_digits,
            path: currency.path,
            groups: currency.groups,
            resolution_paths: currency.resolution_paths,
        })
    }
}

impl Currency {
    pub(super) fn generate<W>(&self, template: &[Token], mut writer: W) -> Result<(), IOError>
    where
        W: Write,
    {
        for token in template {
            writer.write_all(
                match token {
                    &Token::Raw(raw) => raw,
                    Token::Name => &self.name,
                    Token::Ticker => &self.ticker,
                    Token::NormalizedTicker => &self.normalized_ticker,
                    Token::Symbol => &self.symbol,
                }
                .as_bytes(),
            )?;
        }

        Ok(())
    }

    pub(super) fn ticker(&self) -> &String {
        &self.ticker
    }

    pub(super) fn normalized_ticker(&self) -> &String {
        &self.normalized_ticker
    }

    pub(super) fn groups(&self) -> &Vec<String> {
        &self.groups
    }

    pub(super) fn resolution_paths(&self) -> &Vec<Vec<String>> {
        &self.resolution_paths
    }
}

pub(super) enum Token {
    Raw(&'static str),
    Name,
    Ticker,
    NormalizedTicker,
    Symbol,
}

#[derive(Deserialize)]
struct SerializedCurrency {
    #[serde(deserialize_with = "deserialize_non_empty_string")]
    name: String,
    #[serde(deserialize_with = "deserialize_non_empty_string")]
    ticker: String,
    #[serde(deserialize_with = "deserialize_non_empty_string")]
    symbol: String,
    #[serde(deserialize_with = "deserialize_decimal_digits")]
    decimal_digits: u8,
    path: String,
    #[serde(deserialize_with = "deserialize_groups")]
    groups: Vec<String>,
    #[serde(deserialize_with = "deserialize_resolution_paths")]
    resolution_paths: Vec<Vec<String>>,
}

fn serialize_decimal_digits<S>(value: &u8, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(&value.to_string())
}

fn deserialize_non_empty_string<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    let string = String::deserialize(deserializer)?;

    if string.is_empty() {
        return Err(DeserializationError::invalid_length(
            0,
            &"Non-empty string expected!",
        ));
    }

    Ok(string)
}

fn deserialize_decimal_digits<'de, D>(deserializer: D) -> Result<u8, D::Error>
where
    D: Deserializer<'de>,
{
    String::deserialize(deserializer)?
        .parse()
        .map_err(DeserializationError::custom)
}

fn deserialize_groups<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let groups = Vec::<String>::deserialize(deserializer)?;

    if groups.iter().any(String::is_empty) {
        return Err(DeserializationError::invalid_length(
            0,
            &"Non-empty group name expected!",
        ));
    }

    Ok(groups)
}

fn deserialize_resolution_paths<'de, D>(deserializer: D) -> Result<Vec<Vec<String>>, D::Error>
where
    D: Deserializer<'de>,
{
    let resolution_paths = Vec::<Vec<String>>::deserialize(deserializer)?;

    if resolution_paths.is_empty() {
        return Err(DeserializationError::invalid_length(
            0,
            &"At least one resolution path is required!",
        ));
    }

    if resolution_paths.iter().all(|path| path.len() < 2) {
        return Err(DeserializationError::invalid_length(
            resolution_paths.len(),
            &"Resolution paths expect at least 2 currencies!",
        ));
    }

    Ok(resolution_paths)
}
