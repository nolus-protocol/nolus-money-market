use serde::Deserialize;

use super::currency::Currency;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub(crate) struct Network {
    pub name: Box<str>,
    pub currency: Currency,
}
