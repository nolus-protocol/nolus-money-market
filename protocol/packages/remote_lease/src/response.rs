use currency::Group;
use serde::{Deserialize, Serialize};

use finance::coin::CoinDTO;

use remote_lease_wire::{
    coin::WireCoin,
    response::{OperationResponse as WireOperationResponse, SwapResponse as WireSwapResponse},
    ticker::Ticker,
};

pub use remote_lease_wire::{
    lease_id::RemoteLeaseId,
    response::{CloseLeaseResponse, OpenLeaseResponse, TransferOutResponse},
};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(
    deny_unknown_fields,
    rename_all = "snake_case",
    bound(serialize = "", deserialize = "")
)]
pub enum OperationResponse<G>
where
    G: Group,
{
    OpenLease(OpenLeaseResponse),
    CloseLease(CloseLeaseResponse),
    Swap(SwapResponse<G>),
    TransferOut(TransferOutResponse),
}

/// The only response that diverges from its wire twin: it carries a typed
/// `CoinDTO<PaymentGroup>` rather than the stringly-typed `WireCoin`. The other
/// three responses are re-exported verbatim from `remote_lease_wire`.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(
    deny_unknown_fields,
    rename_all = "snake_case",
    bound(serialize = "", deserialize = "")
)]
pub struct SwapResponse<G>
where
    G: Group,
{
    pub amount_out: CoinDTO<G>,
}

// ---------------------------------------------------------------------------
// Typed → wire conversions. See `msg.rs` for the rationale.
// ---------------------------------------------------------------------------

impl<G> From<SwapResponse<G>> for WireSwapResponse
where
    G: Group,
{
    fn from(typed: SwapResponse<G>) -> Self {
        Self {
            amount_out: WireCoin::new(
                typed.amount_out.amount(),
                Ticker::new(typed.amount_out.currency().to_string()),
            ),
        }
    }
}

impl<G> From<OperationResponse<G>> for WireOperationResponse
where
    G: Group,
{
    fn from(typed: OperationResponse<G>) -> Self {
        match typed {
            OperationResponse::OpenLease(r) => Self::OpenLease(r),
            OperationResponse::CloseLease(r) => Self::CloseLease(r),
            OperationResponse::Swap(r) => Self::Swap(r.into()),
            OperationResponse::TransferOut(r) => Self::TransferOut(r),
        }
    }
}
