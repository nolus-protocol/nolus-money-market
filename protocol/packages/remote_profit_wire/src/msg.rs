use serde::{Deserialize, Serialize};

use crate::{coin::WireCoin, error::Error};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub enum Operation {
    OpenProfit(OpenProfitParams),
    Swap(SwapParams),
    TransferOut(TransferOutParams),
    CloseProfit(CloseProfitParams),
}

/// Singleton-establishment payload for the remote profit.
///
/// Unlike the multi-instance remote-lease open, the profit is a singleton
/// selected by port / domain / channel, so there are no per-customer or
/// per-currency establishment fields (no downpayment / lpn / asset tickers).
///
/// `expected_instance_ordinal` is the ADR-0006 replay guard, mirroring the
/// lease ordinal: the Nolus side stamps the ordinal it expects the remote
/// singleton to be established as, so a stale or replayed open against a
/// superseded instance is rejected.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct OpenProfitParams {
    expected_instance_ordinal: u16,
}

impl OpenProfitParams {
    pub const fn new(expected_instance_ordinal: u16) -> Self {
        Self {
            expected_instance_ordinal,
        }
    }

    pub const fn expected_instance_ordinal(&self) -> u16 {
        self.expected_instance_ordinal
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct CloseProfitParams {}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(
    deny_unknown_fields,
    rename_all = "snake_case",
    try_from = "SwapParamsRaw"
)]
pub struct SwapParams {
    coin_in: WireCoin,
    min_out: WireCoin,
}

impl SwapParams {
    pub fn new(coin_in: WireCoin, min_out: WireCoin) -> Result<Self, Error> {
        if coin_in.is_zero() || min_out.is_zero() {
            return Err(Error::ZeroSwapAmount);
        }
        let params = Self { coin_in, min_out };
        params
            .invariant_held()
            .then_some(params)
            .ok_or(Error::SameSwapCurrency)
            .inspect(|p| debug_assert!(p.invariant_held()))
    }

    pub const fn coin_in(&self) -> &WireCoin {
        &self.coin_in
    }

    pub const fn min_out(&self) -> &WireCoin {
        &self.min_out
    }

    pub fn invariant_held(&self) -> bool {
        !self.coin_in.is_zero()
            && !self.min_out.is_zero()
            && self.coin_in.ticker() != self.min_out.ticker()
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
struct SwapParamsRaw {
    coin_in: WireCoin,
    min_out: WireCoin,
}

impl TryFrom<SwapParamsRaw> for SwapParams {
    type Error = Error;

    fn try_from(raw: SwapParamsRaw) -> Result<Self, Self::Error> {
        SwapParams::new(raw.coin_in, raw.min_out)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(
    deny_unknown_fields,
    rename_all = "snake_case",
    try_from = "TransferOutParamsRaw"
)]
pub struct TransferOutParams {
    amount: WireCoin,
}

impl TransferOutParams {
    pub fn new(amount: WireCoin) -> Result<Self, Error> {
        let params = Self { amount };
        params
            .invariant_held()
            .then_some(params)
            .ok_or(Error::ZeroTransferAmount)
            .inspect(|p| debug_assert!(p.invariant_held()))
    }

    pub const fn amount(&self) -> &WireCoin {
        &self.amount
    }

    pub fn invariant_held(&self) -> bool {
        !self.amount.is_zero()
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
struct TransferOutParamsRaw {
    amount: WireCoin,
}

impl TryFrom<TransferOutParamsRaw> for TransferOutParams {
    type Error = Error;

    fn try_from(raw: TransferOutParamsRaw) -> Result<Self, Self::Error> {
        TransferOutParams::new(raw.amount)
    }
}
