use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub(crate) struct Ibc {
    pub network: Box<str>,
    pub currency: Box<str>,
}
