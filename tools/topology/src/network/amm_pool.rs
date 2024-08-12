use serde::Deserialize;

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub(super) struct AmmPool {
    #[serde(rename = "id")]
    _id: String,
    #[serde(rename = "token_0")]
    _token_0: String,
    #[serde(rename = "token_1")]
    _token_1: String,
}
