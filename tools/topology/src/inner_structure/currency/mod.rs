use serde::Deserialize;

pub(crate) use self::{ibc::Ibc, native::Native};

mod ibc;
mod native;

#[derive(Debug, Clone, Deserialize)]
#[serde(from = "self::Raw")]
pub(crate) enum Currency {
    Native(Native),
    Ibc(Ibc),
}

impl From<Raw> for Currency {
    #[inline]
    fn from(Raw { currency, .. }: Raw) -> Self {
        match currency {
            CurrencyRaw::Native(currency) => Self::Native(currency),
            CurrencyRaw::Ibc(currency) => Self::Ibc(currency),
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
struct Raw {
    #[serde(flatten)]
    currency: CurrencyRaw,
    #[serde(rename = "icon")]
    _icon: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
enum CurrencyRaw {
    Native(Native),
    Ibc(Ibc),
}
