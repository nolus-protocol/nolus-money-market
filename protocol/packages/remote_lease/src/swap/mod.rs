use serde::{Deserialize, Serialize};

use currency::Group;
use finance::{coin::CoinDTO, duration::Duration};

use crate::error::Error;

pub use self::build::Builder;

mod build;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(
    deny_unknown_fields,
    rename_all = "snake_case",
    try_from = "SwapParamsRaw<GIn, GOut>"
)]
/// Swap parameters for the remote lease protocol.
///
/// `GIn` is the input-coin group, `GOut` the output (`min_out`) group.
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

    pub(crate) fn invariant_held(&self) -> bool {
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
