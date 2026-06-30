use serde::{Deserialize, Serialize};

use currencies::PaymentGroup;
use finance::coin::CoinDTO;

pub use remote_profit_wire::{
    coin::WireCoin,
    profit_id::RemoteProfitId,
    response::{
        CloseProfitResponse, OpenProfitResponse, OperationResponse as WireOperationResponse,
        SwapResponse as WireSwapResponse, TransferOutResponse,
    },
    ticker::Ticker,
};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub enum OperationResponse {
    OpenProfit(OpenProfitResponse),
    Swap(SwapResponse),
    TransferOut(TransferOutResponse),
    CloseProfit(CloseProfitResponse),
}

/// The only response that diverges from its wire twin: it carries a typed
/// `CoinDTO<PaymentGroup>` rather than the stringly-typed `WireCoin`. The other
/// three responses are re-exported verbatim from `remote_profit_wire`.
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
            OperationResponse::OpenProfit(r) => Self::OpenProfit(r),
            OperationResponse::Swap(r) => Self::Swap(r.into()),
            OperationResponse::TransferOut(r) => Self::TransferOut(r),
            OperationResponse::CloseProfit(r) => Self::CloseProfit(r),
        }
    }
}
