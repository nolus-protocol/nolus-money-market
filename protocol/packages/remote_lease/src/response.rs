use serde::{Deserialize, Serialize};

use currencies::PaymentGroup;
use finance::coin::CoinDTO;

pub use remote_lease_wire::lease_id::RemoteLeaseId;

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
    pub remote_lease_id: RemoteLeaseId,
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

// ---------------------------------------------------------------------------
// Typed → wire conversions. See `msg.rs` for the rationale.
// ---------------------------------------------------------------------------

impl From<&OpenLeaseResponse> for remote_lease_wire::response::OpenLeaseResponse {
    fn from(typed: &OpenLeaseResponse) -> Self {
        Self {
            remote_lease_id: typed.remote_lease_id.clone(),
        }
    }
}

impl From<&SwapResponse> for remote_lease_wire::response::SwapResponse {
    fn from(typed: &SwapResponse) -> Self {
        Self {
            amount_out: remote_lease_wire::coin::WireCoin::new(
                typed.amount_out.amount(),
                remote_lease_wire::ticker::Ticker::new(typed.amount_out.currency().to_string()),
            ),
        }
    }
}

impl From<&OperationResponse> for remote_lease_wire::response::OperationResponse {
    fn from(typed: &OperationResponse) -> Self {
        match typed {
            OperationResponse::OpenLease(r) => Self::OpenLease(r.into()),
            OperationResponse::CloseLease(CloseLeaseResponse {}) => {
                Self::CloseLease(remote_lease_wire::response::CloseLeaseResponse {})
            }
            OperationResponse::Swap(r) => Self::Swap(r.into()),
            OperationResponse::TransferOut(TransferOutResponse {}) => {
                Self::TransferOut(remote_lease_wire::response::TransferOutResponse {})
            }
        }
    }
}
