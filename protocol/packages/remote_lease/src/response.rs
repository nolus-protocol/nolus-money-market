use serde::{Deserialize, Serialize};

use currencies::PaymentGroup;
use finance::coin::CoinDTO;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub enum OperationResponse {
    OpenLease(OpenLeaseResponse),
    CloseLease(CloseLeaseResponse),
    Swap(SwapResponse),
    TransferOut(TransferOutResponse),
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct OpenLeaseResponse {
    pub remote_lease_id: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct CloseLeaseResponse {}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct SwapResponse {
    pub amount_out: CoinDTO<PaymentGroup>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct TransferOutResponse {}
