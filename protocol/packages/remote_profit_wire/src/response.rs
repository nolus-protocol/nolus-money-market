use serde::{Deserialize, Serialize};

use crate::{coin::WireCoin, profit_id::RemoteProfitId};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub enum OperationResponse {
    OpenProfit(OpenProfitResponse),
    Swap(SwapResponse),
    TransferOut(TransferOutResponse),
    CloseProfit(CloseProfitResponse),
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct OpenProfitResponse {
    pub remote_profit_id: RemoteProfitId,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct SwapResponse {
    pub amount_out: WireCoin,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct TransferOutResponse {}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct CloseProfitResponse {}
