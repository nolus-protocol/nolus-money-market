use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
#[serde(try_from = "self::Raw")]
pub(crate) struct Native {
    pub symbol: Box<str>,
    pub decimal_digits: u8,
}

impl TryFrom<Raw> for Native {
    type Error = std::num::ParseIntError;

    fn try_from(
        Raw {
            symbol,
            decimal_digits,
            ..
        }: Raw,
    ) -> Result<Self, Self::Error> {
        decimal_digits.parse().map(|decimal_digits| Self {
            symbol,
            decimal_digits,
        })
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
struct Raw {
    #[serde(rename = "name")]
    _name: Box<str>,
    symbol: Box<str>,
    decimal_digits: Box<str>,
}
