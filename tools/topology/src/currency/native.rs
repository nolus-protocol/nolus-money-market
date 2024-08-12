use serde::Deserialize;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
#[serde(try_from = "self::Raw")]
pub(crate) struct Native {
    symbol: super::Id,
    decimal_digits: u8,
}

impl Native {
    #[inline]
    pub const fn symbol(&self) -> &super::Id {
        &self.symbol
    }

    #[inline]
    pub const fn decimal_digits(&self) -> u8 {
        self.decimal_digits
    }
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
#[serde(deny_unknown_fields, rename_all = "snake_case")]
struct Raw {
    #[serde(rename = "name")]
    _name: String,
    symbol: super::Id,
    decimal_digits: String,
}
