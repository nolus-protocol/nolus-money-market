use std::marker::PhantomData;

use currency::{CurrencyDef, Group, MemberOf};
use platform::batch::Batch;
use sdk::cosmwasm_std::QuerierWrapper;

use crate::{
    SlippageCalculator, SwapTask as SwapTaskT, WithCalculator, error::Result, impl_::trx::SwapTrx,
    swap::ExactAmountIn,
};

pub struct EncodeRequest<'spec, 'querier, SwapTask, SwapClient> {
    spec: &'spec SwapTask,
    querier: QuerierWrapper<'querier>,
    _client: PhantomData<SwapClient>,
}

impl<'spec, 'querier, SwapTask, SwapClient> EncodeRequest<'spec, 'querier, SwapTask, SwapClient> {
    pub fn from(spec: &'spec SwapTask, querier: QuerierWrapper<'querier>) -> Self {
        Self {
            spec,
            querier,
            _client: PhantomData,
        }
    }
}
impl<SwapTask, SwapClient> WithCalculator<SwapTask> for EncodeRequest<'_, '_, SwapTask, SwapClient>
where
    SwapTask: SwapTaskT,
    SwapClient: ExactAmountIn,
{
    type Output = Result<Batch>;

    fn on<CalculatorT>(self, calc: &CalculatorT) -> Self::Output
    where
        CalculatorT: SlippageCalculator<SwapTask::InG>,
        <<CalculatorT as SlippageCalculator<SwapTask::InG>>::OutC as CurrencyDef>::Group:
            MemberOf<<SwapTask::InG as Group>::TopG>,
    {
        let mut filtered = false;

        let swap_trx = SwapTrx::<'_, '_, '_, <SwapTask::InG as Group>::TopG, _>::new(
            self.spec.dex_account(),
            self.spec.oracle(),
            self.querier,
        );

        let out_currency = CalculatorT::OutC::dto().into_super_group();
        super::try_filter_fold_coins(
            self.spec,
            super::not_out_coins_filter(&out_currency),
            swap_trx,
            |mut trx, coin_in| {
                filtered = true;
                calc.min_output(&coin_in, self.querier)
                    .and_then(|min_output| {
                        trx.swap_exact_in::<_, _, SwapClient>(&coin_in, &min_output.into())
                    })
                    .map(|()| trx)
            },
        )
        .inspect(|_| {
            super::expect_at_lease_one_filtered(filtered, &out_currency);
        })
        .map(Into::into)
    }
}
