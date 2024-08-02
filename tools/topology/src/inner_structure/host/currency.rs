use serde::Deserialize;

use super::super::NativeCurrency;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub(crate) struct Currency {
    pub id: String,
    pub native: NativeCurrency,
}
