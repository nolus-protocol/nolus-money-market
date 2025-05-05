use std::marker::PhantomData;

use currency::{CurrencyDef, Group, MemberOf};
use finance::coin::{Amount, Coin, CoinDTO};

use crate::{SlippageCalculator, SwapTask as SwapTaskT};

pub struct AcceptAnyNonZeroSwap<'spec, SwapTask, OutC> {
    spec: &'spec SwapTask,
    _out_c: PhantomData<OutC>,
}

impl<'spec, SwapTask, OutC> AcceptAnyNonZeroSwap<'spec, SwapTask, OutC> {
    pub fn from(spec: &'spec SwapTask) -> Self {
        Self {
            spec,
            _out_c: PhantomData,
        }
    }
}

impl<SwapTask, OutC> SlippageCalculator<SwapTask> for AcceptAnyNonZeroSwap<'_, SwapTask, OutC>
where
    OutC: CurrencyDef,
    SwapTask: SwapTaskT,
{
    type OutC = OutC;

    fn as_spec(&self) -> &SwapTask {
        self.spec
    }

    fn min_output<InG>(&self, _input: &CoinDTO<InG>) -> Coin<Self::OutC>
    where
        InG: Group + MemberOf<SwapTask::InG>,
    {
        // before, it was None on Astroport and "1" on Osmosis.
        const MIN_AMOUNT_OUT: Amount = 1;
        const { Coin::new(MIN_AMOUNT_OUT) }
    }
}
