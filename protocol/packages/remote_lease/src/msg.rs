use serde::{Deserialize, Serialize};

use currencies::PaymentGroup;
use currency::CurrencyDTO;
use finance::coin::CoinDTO;

use crate::error::Error;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub enum LeaseOperationsMsg {
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
    downpayment_currency: CurrencyDTO<PaymentGroup>,
    lpn_currency: CurrencyDTO<PaymentGroup>,
    asset_currency: CurrencyDTO<PaymentGroup>,
}

impl OpenLeaseParams {
    pub fn new(
        expected_instance_ordinal: u16,
        downpayment_currency: CurrencyDTO<PaymentGroup>,
        lpn_currency: CurrencyDTO<PaymentGroup>,
        asset_currency: CurrencyDTO<PaymentGroup>,
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

    pub const fn downpayment_currency(&self) -> &CurrencyDTO<PaymentGroup> {
        &self.downpayment_currency
    }

    pub const fn lpn_currency(&self) -> &CurrencyDTO<PaymentGroup> {
        &self.lpn_currency
    }

    pub const fn asset_currency(&self) -> &CurrencyDTO<PaymentGroup> {
        &self.asset_currency
    }

    pub fn invariant_held(&self) -> bool {
        self.downpayment_currency != self.lpn_currency
            && self.downpayment_currency != self.asset_currency
            && self.lpn_currency != self.asset_currency
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
struct OpenLeaseParamsRaw {
    expected_instance_ordinal: u16,
    downpayment_currency: CurrencyDTO<PaymentGroup>,
    lpn_currency: CurrencyDTO<PaymentGroup>,
    asset_currency: CurrencyDTO<PaymentGroup>,
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
    coin_in: CoinDTO<PaymentGroup>,
    min_out: CoinDTO<PaymentGroup>,
}

impl SwapParams {
    pub fn new(
        coin_in: CoinDTO<PaymentGroup>,
        min_out: CoinDTO<PaymentGroup>,
    ) -> Result<Self, Error> {
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

    pub const fn coin_in(&self) -> &CoinDTO<PaymentGroup> {
        &self.coin_in
    }

    pub const fn min_out(&self) -> &CoinDTO<PaymentGroup> {
        &self.min_out
    }

    pub fn invariant_held(&self) -> bool {
        !self.coin_in.is_zero()
            && !self.min_out.is_zero()
            && self.coin_in.currency() != self.min_out.currency()
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
struct SwapParamsRaw {
    coin_in: CoinDTO<PaymentGroup>,
    min_out: CoinDTO<PaymentGroup>,
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
    amount: CoinDTO<PaymentGroup>,
}

impl TransferOutParams {
    pub fn new(amount: CoinDTO<PaymentGroup>) -> Result<Self, Error> {
        let params = Self { amount };
        params
            .invariant_held()
            .then_some(params)
            .ok_or(Error::ZeroTransferAmount)
            .inspect(|p| debug_assert!(p.invariant_held()))
    }

    pub const fn amount(&self) -> &CoinDTO<PaymentGroup> {
        &self.amount
    }

    pub fn invariant_held(&self) -> bool {
        !self.amount.is_zero()
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
struct TransferOutParamsRaw {
    amount: CoinDTO<PaymentGroup>,
}

impl TryFrom<TransferOutParamsRaw> for TransferOutParams {
    type Error = Error;

    fn try_from(raw: TransferOutParamsRaw) -> Result<Self, Self::Error> {
        TransferOutParams::new(raw.amount)
    }
}
