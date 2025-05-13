use std::marker::PhantomData;

use serde::{Deserialize, Serialize};

use currency::{CurrencyDef, Group};
use finance::coin::{Amount, Coin, CoinDTO};
use sdk::cosmwasm_std::QuerierWrapper;

use crate::{SlippageCalculator, error::Result};

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(bound(serialize = "", deserialize = "",))]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct AcceptAnyNonZeroSwap<InG, OutC> {
    #[serde(skip)]
    _in_g: PhantomData<InG>,
    #[serde(skip)]
    _out_c: PhantomData<OutC>,
}

// cannot use the derived impl since it puts the extra bounds `:Default` on the type args
impl<InG, OutC> Default for AcceptAnyNonZeroSwap<InG, OutC> {
    fn default() -> Self {
        Self {
            _in_g: Default::default(),
            _out_c: Default::default(),
        }
    }
}

impl<InG, OutC> SlippageCalculator<InG> for AcceptAnyNonZeroSwap<InG, OutC>
where
    InG: Group,
    OutC: CurrencyDef,
{
    type OutC = OutC;

    fn min_output(
        &self,
        _input: &CoinDTO<InG>,
        _querier: QuerierWrapper<'_>,
    ) -> Result<Coin<Self::OutC>> {
        // before, it was None on Astroport and "1" on Osmosis.
        const MIN_AMOUNT_OUT: Amount = 1;
        const { Ok(Coin::new(MIN_AMOUNT_OUT)) }
    }
}
