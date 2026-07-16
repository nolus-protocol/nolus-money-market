use serde::{Deserialize, Serialize};

use currency::{CurrencyDTO, Group};
use finance::{coin::CoinDTO, duration::Duration};
use platform::contract::Code;

use crate::error::Error;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub enum ExecuteMsg<LeaseG, LpnG, PaymentG>
where
    LeaseG: Group,
    LpnG: Group,
    PaymentG: Group,
{
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
        params: OpenLeaseParams<LeaseG, LpnG, PaymentG>,
        timeout: Duration,
    },
    /// Outbound `CloseLease` packet. See [`ExecuteMsg::OpenLease`] for `timeout` semantics.
    CloseLease {
        params: CloseLeaseParams,
        timeout: Duration,
    },
    /// Outbound `Swap` packet. See [`ExecuteMsg::OpenLease`] for `timeout` semantics.
    Swap {
        params: SwapParams<PaymentG, PaymentG>,
        timeout: Duration,
    },
    /// Outbound `TransferOut` packet. See [`ExecuteMsg::OpenLease`] for `timeout` semantics.
    TransferOut {
        params: TransferOutParams<PaymentG>,
        timeout: Duration,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub enum Operation<LeaseG, LpnG, PaymentG>
where
    LeaseG: Group,
    LpnG: Group,
    PaymentG: Group,
{
    OpenLease(OpenLeaseParams<LeaseG, LpnG, PaymentG>),
    CloseLease(CloseLeaseParams),
    Swap(SwapParams<PaymentG, PaymentG>),
    TransferOut(TransferOutParams<PaymentG>),
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(
    bound(serialize = "LeaseG: Serialize, LpnG: Serialize, PaymentG: Serialize"),
    deny_unknown_fields,
    rename_all = "snake_case",
    try_from = "OpenLeaseParamsRaw<LeaseG, LpnG, PaymentG>"
)]
pub struct OpenLeaseParams<LeaseG, LpnG, PaymentG>
where
    LeaseG: Group,
    LpnG: Group,
    PaymentG: Group,
{
    expected_instance_ordinal: u16,
    downpayment_currency: CurrencyDTO<PaymentG>,
    lpn_currency: CurrencyDTO<LpnG>,
    asset_currency: CurrencyDTO<LeaseG>,
}

impl<LeaseG, LpnG, PaymentG> OpenLeaseParams<LeaseG, LpnG, PaymentG>
where
    LeaseG: Group,
    LpnG: Group,
    PaymentG: Group,
{
    pub const TIMEOUT: Duration = Duration::from_secs(60);

    pub fn new(
        expected_instance_ordinal: u16,
        downpayment_currency: CurrencyDTO<PaymentG>,
        lpn_currency: CurrencyDTO<LpnG>,
        asset_currency: CurrencyDTO<LeaseG>,
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

    pub const fn downpayment_currency(&self) -> &CurrencyDTO<PaymentG> {
        &self.downpayment_currency
    }

    pub const fn lpn_currency(&self) -> &CurrencyDTO<LpnG> {
        &self.lpn_currency
    }

    pub const fn asset_currency(&self) -> &CurrencyDTO<LeaseG> {
        &self.asset_currency
    }

    pub fn invariant_held(&self) -> bool {
        self.lpn_currency != self.asset_currency
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
struct OpenLeaseParamsRaw<LeaseG, LpnG, PaymentG>
where
    LeaseG: Group,
    LpnG: Group,
    PaymentG: Group,
{
    expected_instance_ordinal: u16,
    downpayment_currency: CurrencyDTO<PaymentG>,
    lpn_currency: CurrencyDTO<LpnG>,
    asset_currency: CurrencyDTO<LeaseG>,
}

impl<LeaseG, LpnG, PaymentG> TryFrom<OpenLeaseParamsRaw<LeaseG, LpnG, PaymentG>>
    for OpenLeaseParams<LeaseG, LpnG, PaymentG>
where
    LeaseG: Group,
    LpnG: Group,
    PaymentG: Group,
{
    type Error = Error;

    fn try_from(raw: OpenLeaseParamsRaw<LeaseG, LpnG, PaymentG>) -> Result<Self, Self::Error> {
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
    try_from = "SwapParamsRaw<GIn, GOut>"
)]
/// Swap parameters for the remote lease protocol.
///
/// `One` carries a single input coin; `Two` carries two input coins.
/// All coins must be non-zero and all currencies pairwise distinct.
pub enum SwapParams<GIn, GOut>
where
    GIn: Group,
    GOut: Group,
{
    One {
        coin_in: CoinDTO<GIn>,
        min_out: CoinDTO<GOut>,
    },
    Two {
        coin_in_1: CoinDTO<GIn>,
        coin_in_2: CoinDTO<GIn>,
        min_out: CoinDTO<GOut>,
    },
}

impl<GIn, GOut> SwapParams<GIn, GOut>
where
    GIn: Group,
    GOut: Group,
{
    pub const TIMEOUT: Duration = Duration::from_secs(300);

    /// Single-input swap. Returns `Err(ZeroSwapAmount)` if either coin is
    /// zero, or `Err(SameSwapCurrency)` if the currencies match.
    pub fn one(coin_in: CoinDTO<GIn>, min_out: CoinDTO<GOut>) -> Result<Self, Error> {
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
        coin_in_1: CoinDTO<GIn>,
        coin_in_2: CoinDTO<GIn>,
        min_out: CoinDTO<GOut>,
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
    pub const fn min_out(&self) -> &CoinDTO<GOut> {
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
enum SwapParamsRaw<GIn, GOut>
where
    GIn: Group,
    GOut: Group,
{
    One {
        coin_in: CoinDTO<GIn>,
        min_out: CoinDTO<GOut>,
    },
    Two {
        coin_in_1: CoinDTO<GIn>,
        coin_in_2: CoinDTO<GIn>,
        min_out: CoinDTO<GOut>,
    },
}

impl<GIn, GOut> TryFrom<SwapParamsRaw<GIn, GOut>> for SwapParams<GIn, GOut>
where
    GIn: Group,
    GOut: Group,
{
    type Error = Error;

    fn try_from(raw: SwapParamsRaw<GIn, GOut>) -> Result<Self, Self::Error> {
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
    try_from = "TransferOutParamsRaw<GOut>"
)]
pub struct TransferOutParams<GOut>
where
    GOut: Group,
{
    amount: CoinDTO<GOut>,
}

impl<GOut> TransferOutParams<GOut>
where
    GOut: Group,
{
    pub const TIMEOUT: Duration = Duration::from_secs(120);

    pub fn new(amount: CoinDTO<GOut>) -> Result<Self, Error> {
        let params = Self { amount };
        params
            .invariant_held()
            .then_some(params)
            .ok_or(Error::ZeroTransferAmount)
            .inspect(|p| debug_assert!(p.invariant_held()))
    }

    pub const fn amount(&self) -> &CoinDTO<GOut> {
        &self.amount
    }

    pub fn invariant_held(&self) -> bool {
        !self.amount.is_zero()
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
struct TransferOutParamsRaw<GOut>
where
    GOut: Group,
{
    amount: CoinDTO<GOut>,
}

impl<GOut> TryFrom<TransferOutParamsRaw<GOut>> for TransferOutParams<GOut>
where
    GOut: Group,
{
    type Error = Error;

    fn try_from(raw: TransferOutParamsRaw<GOut>) -> Result<Self, Self::Error> {
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

fn wire_ticker<G>(currency: &CurrencyDTO<G>) -> remote_lease_wire::ticker::Ticker
where
    G: Group,
{
    remote_lease_wire::ticker::Ticker::new(currency.to_string())
}

fn wire_coin<G>(coin: &CoinDTO<G>) -> remote_lease_wire::coin::WireCoin
where
    G: Group,
{
    remote_lease_wire::coin::WireCoin::new(coin.amount(), wire_ticker(&coin.currency()))
}

impl<LeaseG, LpnG, PaymentG> From<&OpenLeaseParams<LeaseG, LpnG, PaymentG>>
    for remote_lease_wire::msg::OpenLeaseParams
where
    LeaseG: Group,
    LpnG: Group,
    PaymentG: Group,
{
    fn from(typed: &OpenLeaseParams<LeaseG, LpnG, PaymentG>) -> Self {
        Self::new(
            typed.expected_instance_ordinal(),
            wire_ticker(typed.downpayment_currency()),
            wire_ticker(typed.lpn_currency()),
            wire_ticker(typed.asset_currency()),
        )
        .expect("typed OpenLeaseParams already upholds the pairwise-distinct invariant")
    }
}

impl<GIn, GOut> From<&SwapParams<GIn, GOut>> for remote_lease_wire::msg::SwapParams
where
    GIn: Group,
    GOut: Group,
{
    fn from(typed: &SwapParams<GIn, GOut>) -> Self {
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

impl<GOut> From<&TransferOutParams<GOut>> for remote_lease_wire::msg::TransferOutParams
where
    GOut: Group,
{
    fn from(typed: &TransferOutParams<GOut>) -> Self {
        Self::new(wire_coin(typed.amount()))
            .expect("typed TransferOutParams already upholds the non-zero invariant")
    }
}

impl<LeaseG, Lpns, PaymentG> From<&Operation<LeaseG, Lpns, PaymentG>>
    for remote_lease_wire::msg::Operation
where
    LeaseG: Group,
    Lpns: Group,
    PaymentG: Group,
{
    fn from(typed: &Operation<LeaseG, Lpns, PaymentG>) -> Self {
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

    type SwapP2P = SwapParams<PaymentGroup, PaymentGroup>;
    type ExecuteP2P = ExecuteMsg<PaymentGroup, PaymentGroup, PaymentGroup>;
    type OpenLeaseP2P = OpenLeaseParams<PaymentGroup, PaymentGroup, PaymentGroup>;
    type TransferOutP2P = TransferOutParams<PaymentGroup>;

    // Each variant of `ExecuteMsg` carries its own serde encoding, so a
    // regression in one variant's attributes (`rename_all`, the tuple-vs-struct
    // shape) is invisible to the others. Every variant therefore gets its own
    // wire-shape assertion; the inner param types are byte-pinned separately by
    // `tests/cross_surface.rs`.

    #[test]
    fn open_channel_wire_shape() {
        assert_eq!(
            serde_json::json!([]),
            variant_body("open_channel", &ExecuteP2P::OpenChannel())
        );
    }

    #[test]
    fn close_channel_wire_shape() {
        assert_eq!(
            serde_json::json!([]),
            variant_body("close_channel", &ExecuteP2P::CloseChannel())
        );
    }

    #[test]
    fn new_lease_code_wire_shape() {
        let msg = ExecuteP2P::NewLeaseCode {
            lease_code: Code::unchecked(20),
        };
        assert_struct_fields(&["lease_code"], &variant_body("new_lease_code", &msg));
    }

    #[test]
    fn open_lease_wire_shape() {
        let msg = ExecuteP2P::OpenLease {
            params: open_lease_params(),
            timeout: OpenLeaseP2P::TIMEOUT,
        };
        assert_struct_fields(&["params", "timeout"], &variant_body("open_lease", &msg));
    }

    #[test]
    fn close_lease_wire_shape() {
        let msg = ExecuteP2P::CloseLease {
            params: CloseLeaseParams {},
            timeout: CloseLeaseParams::TIMEOUT,
        };
        assert_struct_fields(&["params", "timeout"], &variant_body("close_lease", &msg));
    }

    #[test]
    fn swap_wire_shape() {
        let msg = ExecuteP2P::Swap {
            params: SwapP2P::one(
                Coin::<PaymentC1>::new(1000).into(),
                Coin::<PaymentC2>::new(42).into(),
            )
            .expect("distinct non-zero amounts"),
            timeout: SwapP2P::TIMEOUT,
        };
        assert_struct_fields(&["params", "timeout"], &variant_body("swap", &msg));
    }

    #[test]
    fn transfer_out_wire_shape() {
        let msg = ExecuteP2P::TransferOut {
            params: TransferOutParams::new(Coin::<PaymentC3>::new(1000).into())
                .expect("non-zero amount"),
            timeout: TransferOutP2P::TIMEOUT,
        };
        assert_struct_fields(&["params", "timeout"], &variant_body("transfer_out", &msg));
    }

    fn open_lease_params() -> OpenLeaseP2P {
        OpenLeaseParams::new(
            7,
            currency::dto::<PaymentC1, _>(),
            currency::dto::<PaymentC2, _>(),
            currency::dto::<PaymentC3, _>(),
        )
        .expect("three distinct currencies")
    }

    fn variant_body(expected_tag: &str, msg: &ExecuteP2P) -> serde_json::Value {
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
