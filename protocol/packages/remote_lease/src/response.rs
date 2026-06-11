use serde::{Deserialize, Serialize};

use currencies::PaymentGroup;
use finance::coin::CoinDTO;

pub use remote_lease_wire::{
    coin::WireCoin,
    lease_id::RemoteLeaseId,
    response::{
        CloseLeaseResponse, OpenLeaseResponse, OperationResponse as WireOperationResponse,
        SwapResponse as WireSwapResponse, TransferOutResponse,
    },
    ticker::Ticker,
};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub enum OperationResponse {
    OpenLease(OpenLeaseResponse),
    CloseLease(CloseLeaseResponse),
    Swap(SwapResponse),
    TransferOut(TransferOutResponse),
}

/// The only response that diverges from its wire twin: it carries a typed
/// `CoinDTO<PaymentGroup>` rather than the stringly-typed `WireCoin`. The other
/// three responses are re-exported verbatim from `remote_lease_wire`.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct SwapResponse {
    pub amount_out: CoinDTO<PaymentGroup>,
}

// ---------------------------------------------------------------------------
// Typed → wire conversions. See `msg.rs` for the rationale.
// ---------------------------------------------------------------------------

impl From<SwapResponse> for WireSwapResponse {
    fn from(typed: SwapResponse) -> Self {
        Self {
            amount_out: WireCoin::new(
                typed.amount_out.amount(),
                Ticker::new(typed.amount_out.currency().to_string()),
            ),
        }
    }
}

impl From<OperationResponse> for WireOperationResponse {
    fn from(typed: OperationResponse) -> Self {
        match typed {
            OperationResponse::OpenLease(r) => Self::OpenLease(r),
            OperationResponse::CloseLease(r) => Self::CloseLease(r),
            OperationResponse::Swap(r) => Self::Swap(r.into()),
            OperationResponse::TransferOut(r) => Self::TransferOut(r),
        }
    }
}
