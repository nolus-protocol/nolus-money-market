use dex::SlippageCalculator;
use finance::{
    coin::{Coin, CoinDTO},
    percent::Percent,
};
use serde::{Deserialize, Serialize};

use crate::{
    api::LeaseAssetCurrencies, contract::state::opened::close::Calculator, finance::LpnCurrency,
};

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "skel_testing", derive(Debug))]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct MaxSlippage {
    max_slippage: Percent,
}

impl MaxSlippage {
    pub fn with(max_slippage: Percent) -> Self {
        Self { max_slippage }
    }
}

impl SlippageCalculator<LeaseAssetCurrencies> for MaxSlippage {
    type OutC = LpnCurrency;

    fn min_output(&self, _input: &CoinDTO<LeaseAssetCurrencies>) -> Coin<Self::OutC> {
        todo!("TODO use oracle_platform::convert::{{from|to}}_quote(..)")
    }
}
impl Calculator for MaxSlippage {}
