use serde::{Deserialize, Serialize};

use crate::response::OperationResponse;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub enum RemoteLeaseCallback {
    OperationOk(OperationResponse),
    OperationErr(String),
    OperationTimeout,
}
