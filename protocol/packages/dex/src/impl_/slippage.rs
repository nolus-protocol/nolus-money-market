use std::marker::PhantomData;

use currency::{CurrencyDef, Group, MemberOf};
use finance::coin::{Amount, Coin, CoinDTO};

use crate::{SlippageCalculator, SwapTask as SwapTaskT};

pub struct AcceptAnyNonZeroSwap<SwapTask, OutC> {
    _spec: PhantomData<SwapTask>,
    _out_c: PhantomData<OutC>,
}

// cannot use the derived impl since it puts the extra bounds `:Default` on the type args
impl<SwapTask, OutC> Default for AcceptAnyNonZeroSwap<SwapTask, OutC> {
    fn default() -> Self {
        Self {
            _spec: Default::default(),
            _out_c: Default::default(),
        }
    }
}

impl<SwapTask, OutC> SlippageCalculator<SwapTask> for AcceptAnyNonZeroSwap<SwapTask, OutC>
where
    OutC: CurrencyDef,
    SwapTask: SwapTaskT,
{
    type OutC = OutC;

    fn min_output<InG>(&self, _input: &CoinDTO<InG>) -> Coin<Self::OutC>
    where
        InG: Group + MemberOf<SwapTask::InG>,
    {
        // before, it was None on Astroport and "1" on Osmosis.
        const MIN_AMOUNT_OUT: Amount = 1;
        const { Coin::new(MIN_AMOUNT_OUT) }
    }
}
