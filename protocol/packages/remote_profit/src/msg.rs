use serde::{Deserialize, Serialize};

use currencies::PaymentGroup;
use currency::CurrencyDTO;
use finance::{coin::CoinDTO, duration::Duration};

pub use remote_profit_wire::nolus_receiver::NolusReceiver;

use crate::error::Error;

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
///
/// `nolus_receiver` is the store-once drain receiver — the Nolus address the
/// funded profit ultimately drains into. It is committed to the Solana side at
/// `open_profit` so subsequent `TransferOut` packets stay amount-only, the
/// recipient derived from this stored value rather than re-sent per packet.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct OpenProfitParams {
    expected_instance_ordinal: u16,
    nolus_receiver: NolusReceiver,
}

impl OpenProfitParams {
    pub const TIMEOUT: Duration = Duration::from_secs(60);

    pub const fn new(expected_instance_ordinal: u16, nolus_receiver: NolusReceiver) -> Self {
        Self {
            expected_instance_ordinal,
            nolus_receiver,
        }
    }

    pub const fn expected_instance_ordinal(&self) -> u16 {
        self.expected_instance_ordinal
    }

    pub const fn nolus_receiver(&self) -> &NolusReceiver {
        &self.nolus_receiver
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct CloseProfitParams {}

impl CloseProfitParams {
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

fn wire_ticker(currency: &CurrencyDTO<PaymentGroup>) -> remote_profit_wire::ticker::Ticker {
    remote_profit_wire::ticker::Ticker::new(currency.to_string())
}

fn wire_coin(coin: &CoinDTO<PaymentGroup>) -> remote_profit_wire::coin::WireCoin {
    remote_profit_wire::coin::WireCoin::new(coin.amount(), wire_ticker(&coin.currency()))
}

impl From<&OpenProfitParams> for remote_profit_wire::msg::OpenProfitParams {
    fn from(typed: &OpenProfitParams) -> Self {
        Self::new(
            typed.expected_instance_ordinal(),
            typed.nolus_receiver().clone(),
        )
    }
}

impl From<&SwapParams> for remote_profit_wire::msg::SwapParams {
    fn from(typed: &SwapParams) -> Self {
        Self::new(wire_coin(typed.coin_in()), wire_coin(typed.min_out()))
            .expect("typed SwapParams already upholds the non-zero distinct-currency invariant")
    }
}

impl From<&TransferOutParams> for remote_profit_wire::msg::TransferOutParams {
    fn from(typed: &TransferOutParams) -> Self {
        Self::new(wire_coin(typed.amount()))
            .expect("typed TransferOutParams already upholds the non-zero invariant")
    }
}

impl From<&Operation> for remote_profit_wire::msg::Operation {
    fn from(typed: &Operation) -> Self {
        match typed {
            Operation::OpenProfit(p) => Self::OpenProfit(p.into()),
            Operation::Swap(p) => Self::Swap(p.into()),
            Operation::TransferOut(p) => Self::TransferOut(p.into()),
            Operation::CloseProfit(CloseProfitParams {}) => {
                Self::CloseProfit(remote_profit_wire::msg::CloseProfitParams {})
            }
        }
    }
}
