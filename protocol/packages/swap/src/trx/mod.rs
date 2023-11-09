#[cfg(feature = "osmosis")]
mod osmosis;

#[cfg(feature = "testing")]
pub mod test;
#[cfg(feature = "testing")]
pub use test::*;

use currency::{self, Group};
use finance::coin::{Amount, CoinDTO};
use platform::{ica::HostAccount, trx::Transaction};
use sdk::cosmos_sdk_proto::cosmos::base::abci::v1beta1::MsgData;

use crate::{error::Result, SwapPath};

pub fn exact_amount_in() -> impl ExactAmountIn {
    #[cfg(feature = "osmosis")]
    {
        osmosis::Impl {}
    }
}

pub trait ExactAmountIn {
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
