use serde::{Deserialize, Serialize};

use currencies::PaymentGroup;
use currency::CurrencyDTO;
use finance::{coin::CoinDTO, duration::Duration};
use platform::contract::Code;

use crate::error::Error;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub enum ExecuteMsg {
    /// Initiate the channel handshake. Allowed only when no channel is recorded.
    OpenChannel(),
    /// Begin closing the recorded channel. Allowed only when it is currently `Open`.
    CloseChannel(),
    NewLeaseCode {
        // This is an internal system API and we use [Code]
        lease_code: Code,
    },
    /// Outbound `OpenLease` packet. Caller must be an instance of `Config.lease_code`.
    /// `timeout` is the relative duration after which the ICS-04 packet expires;
    /// the controller anchors it to its own block time at send.
    OpenLease {
        params: OpenLeaseParams,
        timeout: Duration,
    },
    /// Outbound `CloseLease` packet. See [`ExecuteMsg::OpenLease`] for `timeout` semantics.
    CloseLease {
        params: CloseLeaseParams,
        timeout: Duration,
    },
    /// Outbound `Swap` packet. See [`ExecuteMsg::OpenLease`] for `timeout` semantics.
    Swap {
        params: SwapParams,
        timeout: Duration,
    },
    /// Outbound `TransferOut` packet. See [`ExecuteMsg::OpenLease`] for `timeout` semantics.
    TransferOut {
        params: TransferOutParams,
        timeout: Duration,
    },
}

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
/// Swap parameters for the remote lease protocol.
///
/// `One` carries a single input coin; `Two` carries two input coins.
/// All coins must be non-zero and all currencies pairwise distinct.
pub enum SwapParams {
    One {
        coin_in: CoinDTO<PaymentGroup>,
        min_out: CoinDTO<PaymentGroup>,
    },
    Two {
        coin_in_1: CoinDTO<PaymentGroup>,
        coin_in_2: CoinDTO<PaymentGroup>,
        min_out: CoinDTO<PaymentGroup>,
    },
}

impl SwapParams {
    pub const TIMEOUT: Duration = Duration::from_secs(300);

    /// Single-input swap. Returns `Err(ZeroSwapAmount)` if either coin is
    /// zero, or `Err(SameSwapCurrency)` if the currencies match.
    pub fn one(
        coin_in: CoinDTO<PaymentGroup>,
        min_out: CoinDTO<PaymentGroup>,
    ) -> Result<Self, Error> {
        if coin_in.is_zero() || min_out.is_zero() {
            return Err(Error::ZeroSwapAmount);
        }
        if coin_in.currency() == min_out.currency() {
            return Err(Error::SameSwapCurrency);
        }
        let params = Self::One { coin_in, min_out };
        debug_assert!(params.invariant_held());
        Ok(params)
    }

    /// Two-input swap. Returns `Err(ZeroSwapAmount)` if any coin is zero,
    /// `Err(SameSwapCurrency)` if an input currency equals the output currency,
    /// or `Err(DuplicateSwapInputCurrency)` if the two input currencies match.
    pub fn two(
        coin_in_1: CoinDTO<PaymentGroup>,
        coin_in_2: CoinDTO<PaymentGroup>,
        min_out: CoinDTO<PaymentGroup>,
    ) -> Result<Self, Error> {
        if coin_in_1.is_zero() || coin_in_2.is_zero() || min_out.is_zero() {
            return Err(Error::ZeroSwapAmount);
        }
        if coin_in_1.currency() == min_out.currency() || coin_in_2.currency() == min_out.currency()
        {
            return Err(Error::SameSwapCurrency);
        }
        if coin_in_1.currency() == coin_in_2.currency() {
            return Err(Error::DuplicateSwapInputCurrency);
        }
        let params = Self::Two {
            coin_in_1,
            coin_in_2,
            min_out,
        };
        debug_assert!(params.invariant_held());
        Ok(params)
    }

    /// Returns the minimum output coin, common to both variants.
    pub const fn min_out(&self) -> &CoinDTO<PaymentGroup> {
        match self {
            Self::One { min_out, .. } | Self::Two { min_out, .. } => min_out,
        }
    }

    pub fn invariant_held(&self) -> bool {
        match self {
            Self::One { coin_in, min_out } => {
                !coin_in.is_zero() && !min_out.is_zero() && coin_in.currency() != min_out.currency()
            }
            Self::Two {
                coin_in_1,
                coin_in_2,
                min_out,
            } => {
                !coin_in_1.is_zero()
                    && !coin_in_2.is_zero()
                    && !min_out.is_zero()
                    && coin_in_1.currency() != min_out.currency()
                    && coin_in_2.currency() != min_out.currency()
                    && coin_in_1.currency() != coin_in_2.currency()
            }
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
enum SwapParamsRaw {
    One {
        coin_in: CoinDTO<PaymentGroup>,
        min_out: CoinDTO<PaymentGroup>,
    },
    Two {
        coin_in_1: CoinDTO<PaymentGroup>,
        coin_in_2: CoinDTO<PaymentGroup>,
        min_out: CoinDTO<PaymentGroup>,
    },
}

impl TryFrom<SwapParamsRaw> for SwapParams {
    type Error = Error;

    fn try_from(raw: SwapParamsRaw) -> Result<Self, Self::Error> {
        match raw {
            SwapParamsRaw::One { coin_in, min_out } => SwapParams::one(coin_in, min_out),
            SwapParamsRaw::Two {
                coin_in_1,
                coin_in_2,
                min_out,
            } => SwapParams::two(coin_in_1, coin_in_2, min_out),
        }
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
        .expect("typed OpenLeaseParams already upholds the pairwise-distinct invariant")
    }
}

impl From<&SwapParams> for remote_lease_wire::msg::SwapParams {
    fn from(typed: &SwapParams) -> Self {
        match typed {
            SwapParams::One { coin_in, min_out } => {
                Self::one(wire_coin(coin_in), wire_coin(min_out))
            }
            SwapParams::Two {
                coin_in_1,
                coin_in_2,
                min_out,
            } => Self::two(
                wire_coin(coin_in_1),
                wire_coin(coin_in_2),
                wire_coin(min_out),
            ),
        }
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

#[cfg(test)]
mod tests {
    use currencies::{
        PaymentGroup,
        testing::{PaymentC1, PaymentC2, PaymentC3},
    };
    use finance::coin::Coin;
    use platform::contract::Code;

    use super::{CloseLeaseParams, ExecuteMsg, OpenLeaseParams, SwapParams, TransferOutParams};

    // Each variant of `ExecuteMsg` carries its own serde encoding, so a
    // regression in one variant's attributes (`rename_all`, the tuple-vs-struct
    // shape) is invisible to the others. Every variant therefore gets its own
    // wire-shape assertion; the inner param types are byte-pinned separately by
    // `tests/cross_surface.rs`.

    #[test]
    fn open_channel_wire_shape() {
        assert_eq!(
            serde_json::json!([]),
            variant_body("open_channel", &ExecuteMsg::OpenChannel())
        );
    }

    #[test]
    fn close_channel_wire_shape() {
        assert_eq!(
            serde_json::json!([]),
            variant_body("close_channel", &ExecuteMsg::CloseChannel())
        );
    }

    #[test]
    fn new_lease_code_wire_shape() {
        let msg = ExecuteMsg::NewLeaseCode {
            lease_code: Code::unchecked(20),
        };
        assert_struct_fields(&["lease_code"], &variant_body("new_lease_code", &msg));
    }

    #[test]
    fn open_lease_wire_shape() {
        let msg = ExecuteMsg::OpenLease {
            params: open_lease_params(),
            timeout: OpenLeaseParams::TIMEOUT,
        };
        assert_struct_fields(&["params", "timeout"], &variant_body("open_lease", &msg));
    }

    #[test]
    fn close_lease_wire_shape() {
        let msg = ExecuteMsg::CloseLease {
            params: CloseLeaseParams {},
            timeout: CloseLeaseParams::TIMEOUT,
        };
        assert_struct_fields(&["params", "timeout"], &variant_body("close_lease", &msg));
    }

    #[test]
    fn swap_wire_shape() {
        let msg = ExecuteMsg::Swap {
            params: SwapParams::one(
                Coin::<PaymentC1>::new(1000).into(),
                Coin::<PaymentC2>::new(42).into(),
            )
            .expect("distinct non-zero amounts"),
            timeout: SwapParams::TIMEOUT,
        };
        assert_struct_fields(&["params", "timeout"], &variant_body("swap", &msg));
    }

    #[test]
    fn transfer_out_wire_shape() {
        let msg = ExecuteMsg::TransferOut {
            params: TransferOutParams::new(Coin::<PaymentC3>::new(1000).into())
                .expect("non-zero amount"),
            timeout: TransferOutParams::TIMEOUT,
        };
        assert_struct_fields(&["params", "timeout"], &variant_body("transfer_out", &msg));
    }

    fn open_lease_params() -> OpenLeaseParams {
        OpenLeaseParams::new(
            7,
            currency::dto::<PaymentC1, PaymentGroup>(),
            currency::dto::<PaymentC2, PaymentGroup>(),
            currency::dto::<PaymentC3, PaymentGroup>(),
        )
        .expect("three distinct currencies")
    }

    fn variant_body(expected_tag: &str, msg: &ExecuteMsg) -> serde_json::Value {
        let json: serde_json::Value =
            serde_json::to_value(msg).expect("serialization must succeed");
        let mut object = json
            .as_object()
            .expect("externally-tagged variant must be an object")
            .clone();
        assert_eq!(1, object.len(), "exactly one variant tag");
        object
            .remove(expected_tag)
            .expect("variant tag must match its snake_case name")
    }

    fn assert_struct_fields(expected_fields: &[&str], body: &serde_json::Value) {
        let object = body
            .as_object()
            .expect("struct-style variant body must be an object");
        assert_eq!(expected_fields.len(), object.len());
        expected_fields
            .iter()
            .for_each(|field| assert!(object.contains_key(*field), "missing field {field}"));
    }
}
