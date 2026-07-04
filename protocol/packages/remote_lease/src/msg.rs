use serde::{Deserialize, Serialize};

use currencies::PaymentGroup;
use currency::CurrencyDTO;
use finance::{coin::CoinDTO, duration::Duration};

use crate::error::Error;

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
    downpayment_currency: CurrencyDTO<PaymentGroup>,
    lpn_currency: CurrencyDTO<PaymentGroup>,
    asset_currency: CurrencyDTO<PaymentGroup>,
}

impl OpenLeaseParams {
    pub const TIMEOUT: Duration = Duration::from_secs(60);

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
        self.lpn_currency != self.asset_currency
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

impl CloseLeaseParams {
    pub const TIMEOUT: Duration = Duration::from_secs(60);
}

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
    pub const TIMEOUT: Duration = Duration::from_secs(300);

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
    pub const TIMEOUT: Duration = Duration::from_secs(120);

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

// ---------------------------------------------------------------------------
// Typed → wire conversions.
//
// These are infallible because the typed surface enforces the same invariants
// the wire surface enforces (distinct currencies, non-zero amounts), so the
// stringly-typed wire constructors cannot fail given a valid typed input.
// The cross-surface integration test verifies byte-identical JSON for each
// variant.
// ---------------------------------------------------------------------------

fn wire_ticker(currency: &CurrencyDTO<PaymentGroup>) -> remote_lease_wire::ticker::Ticker {
    remote_lease_wire::ticker::Ticker::new(currency.to_string())
}

fn wire_coin(coin: &CoinDTO<PaymentGroup>) -> remote_lease_wire::coin::WireCoin {
    remote_lease_wire::coin::WireCoin::new(coin.amount(), wire_ticker(&coin.currency()))
}

impl From<&OpenLeaseParams> for remote_lease_wire::msg::OpenLeaseParams {
    fn from(typed: &OpenLeaseParams) -> Self {
        Self::new(
            typed.expected_instance_ordinal(),
            wire_ticker(typed.downpayment_currency()),
            wire_ticker(typed.lpn_currency()),
            wire_ticker(typed.asset_currency()),
        )
        .expect("typed OpenLeaseParams already upholds the distinct lpn/asset-currency invariant")
    }
}

impl From<&SwapParams> for remote_lease_wire::msg::SwapParams {
    fn from(typed: &SwapParams) -> Self {
        Self::new(wire_coin(typed.coin_in()), wire_coin(typed.min_out()))
            .expect("typed SwapParams already upholds the non-zero distinct-currency invariant")
    }
}

impl From<&TransferOutParams> for remote_lease_wire::msg::TransferOutParams {
    fn from(typed: &TransferOutParams) -> Self {
        Self::new(wire_coin(typed.amount()))
            .expect("typed TransferOutParams already upholds the non-zero invariant")
    }
}

impl From<&Operation> for remote_lease_wire::msg::Operation {
    fn from(typed: &Operation) -> Self {
        match typed {
            Operation::OpenLease(p) => Self::OpenLease(p.into()),
            Operation::CloseLease(CloseLeaseParams {}) => {
                Self::CloseLease(remote_lease_wire::msg::CloseLeaseParams {})
            }
            Operation::Swap(p) => Self::Swap(p.into()),
            Operation::TransferOut(p) => Self::TransferOut(p.into()),
        }
    }
}
