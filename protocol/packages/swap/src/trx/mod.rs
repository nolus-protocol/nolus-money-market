use currency::Group;
use finance::coin::{Amount, CoinDTO};
use platform::{ica::HostAccount, trx::Transaction};
use sdk::cosmos_sdk_proto::Any;

#[cfg(feature = "astroport")]
use self::astroport as impl_mod;
#[cfg(feature = "testing")]
pub use self::impl_mod::{RequestMsg, ResponseMsg};
#[cfg(feature = "osmosis")]
use self::osmosis as impl_mod;

use crate::{error::Result, SwapPath};

#[cfg(feature = "astroport")]
mod astroport;
#[cfg(feature = "osmosis")]
mod osmosis;

pub trait TypeUrl {
    const TYPE_URL: &'static str;
}

pub fn exact_amount_in() -> impl ExactAmountIn {
    impl_mod::Impl
}

pub trait ExactAmountIn {
    /// `swap_path` should be a non-empty list
    ///
    /// `GIn` - the group of the input token
    /// `GSwap` - the group common for all tokens in the swap path
    fn build<GIn, GSwap>(
        &self,
        trx: &mut Transaction,
        sender: HostAccount,
        token_in: &CoinDTO<GIn>,
        swap_path: &SwapPath,
    ) -> Result<()>
    where
        GIn: Group,
        GSwap: Group;

    fn parse<I>(&self, trx_resps: &mut I) -> Result<Amount>
    where
        I: Iterator<Item = Any>;

    #[cfg(any(test, feature = "testing"))]
    fn build_resp(&self, amount_out: Amount) -> Any;
}
