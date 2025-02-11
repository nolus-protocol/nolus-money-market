use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct EmptyMsg {}

#[derive(Serialize, Deserialize, Clone, Eq, PartialEq)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub enum QueryMsg {
    /// Implementation of [versioning::query::ProtocolPackage::Release]
    ProtocolPackageRelease {},
}
