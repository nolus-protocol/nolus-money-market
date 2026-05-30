use serde::{Deserialize, Serialize};

use crate::{coin::WireCoin, error::Error, ticker::Ticker};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub enum Operation {
    OpenLease(OpenLeaseParams),
    CloseLease(CloseLeaseParams),
    Swap(SwapParams),
    TransferOut(TransferOutParams),
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(
    deny_unknown_fields,
    rename_all = "snake_case",
    try_from = "OpenLeaseParamsRaw"
)]
pub struct OpenLeaseParams {
    expected_instance_ordinal: u16,
    downpayment_currency: Ticker,
    lpn_currency: Ticker,
    asset_currency: Ticker,
}

impl OpenLeaseParams {
    pub fn new(
        expected_instance_ordinal: u16,
        downpayment_currency: Ticker,
        lpn_currency: Ticker,
        asset_currency: Ticker,
    ) -> Result<Self, Error> {
        let params = Self {
            expected_instance_ordinal,
            downpayment_currency,
            lpn_currency,
            asset_currency,
        };
        params
            .invariant_held()
            .then_some(params)
            .ok_or(Error::DuplicateLeaseCurrencies)
            .inspect(|p| debug_assert!(p.invariant_held()))
    }

    pub const fn expected_instance_ordinal(&self) -> u16 {
        self.expected_instance_ordinal
    }

    pub const fn downpayment_currency(&self) -> &Ticker {
        &self.downpayment_currency
    }

    pub const fn lpn_currency(&self) -> &Ticker {
        &self.lpn_currency
    }

    pub const fn asset_currency(&self) -> &Ticker {
        &self.asset_currency
    }

    pub fn invariant_held(&self) -> bool {
        self.lpn_currency != self.asset_currency
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
struct OpenLeaseParamsRaw {
    expected_instance_ordinal: u16,
    downpayment_currency: Ticker,
    lpn_currency: Ticker,
    asset_currency: Ticker,
}

impl TryFrom<OpenLeaseParamsRaw> for OpenLeaseParams {
    type Error = Error;

    fn try_from(raw: OpenLeaseParamsRaw) -> Result<Self, Self::Error> {
        OpenLeaseParams::new(
            raw.expected_instance_ordinal,
            raw.downpayment_currency,
            raw.lpn_currency,
            raw.asset_currency,
        )
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct CloseLeaseParams {}

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
