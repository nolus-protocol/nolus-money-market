use currency::Group;
use finance::coin::{Amount, CoinDTO};
use platform::{ica::HostAccount, trx::Transaction};
use sdk::cosmos_sdk_proto::cosmos::base::abci::v1beta1::MsgData;

#[cfg(feature = "testing")]
pub use self::test::*;

use crate::{error::Result, SwapPath};

#[cfg(feature = "testing")]
pub mod test;

#[cfg(feature = "astroport")]
mod astroport;
#[cfg(feature = "astroport")]
use self::astroport as impl_mod;
#[cfg(feature = "osmosis")]
mod osmosis;
#[cfg(feature = "osmosis")]
use self::osmosis as impl_mod;

pub fn exact_amount_in() -> impl ExactAmountIn {
    impl_mod::Impl
}

pub trait ExactAmountIn {
    /// `swap_path` should be a non-empty list
    fn build<G>(
        &self,
        trx: &mut Transaction,
        sender: HostAccount,
        token_in: &CoinDTO<G>,
        swap_path: &SwapPath,
    ) -> Result<()>
    where
        G: Group;

    fn parse<I>(&self, trx_resps: &mut I) -> Result<Amount>
    where
        I: Iterator<Item = MsgData>;

    #[cfg(any(test, feature = "testing"))]
    fn build_resp(&self, amount_out: Amount) -> MsgData;
}
