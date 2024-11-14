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

impl From<Raw> for Native {
    #[inline]
    fn from(
        Raw {
            symbol,
            decimal_digits,
            ..
        }: Raw,
    ) -> Self {
        Self {
            symbol,
            decimal_digits,
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
struct Raw {
    #[serde(rename = "name")]
    _name: String,
    symbol: super::Id,
    decimal_digits: u8,
}
