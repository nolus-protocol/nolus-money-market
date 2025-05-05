use std::marker::PhantomData;

use currency::{CurrencyDef, Group, MemberOf};
use platform::batch::Batch;
use sdk::cosmwasm_std::QuerierWrapper;

use crate::{
    SlippageCalculator, SwapTask as SwapTaskT, WithCalculator, error::Result, impl_::trx::SwapTrx,
    swap::ExactAmountIn,
};

pub struct EncodeRequest<'querier, SwapTask, SwapClient> {
    querier: QuerierWrapper<'querier>,
    _spec: PhantomData<SwapTask>,
    _client: PhantomData<SwapClient>,
}

impl<'querier, SwapTask, SwapClient> EncodeRequest<'querier, SwapTask, SwapClient> {
    pub fn from(querier: QuerierWrapper<'querier>) -> Self {
        Self {
            querier,
            _spec: PhantomData,
            _client: PhantomData,
        }
    }
}
impl<SwapTask, SwapClient> WithCalculator<SwapTask> for EncodeRequest<'_, SwapTask, SwapClient>
where
    SwapTask: SwapTaskT,
    SwapClient: ExactAmountIn,
{
    type Output = Result<Batch>;

    fn on<CalculatorT>(self, calc: CalculatorT) -> Self::Output
    where
        CalculatorT: SlippageCalculator<SwapTask>,
        <<CalculatorT as SlippageCalculator<SwapTask>>::OutC as CurrencyDef>::Group:
            MemberOf<<SwapTask::InG as Group>::TopG>,
    {
        let mut filtered = false;

        let swap_trx = SwapTrx::<'_, '_, '_, <SwapTask::InG as Group>::TopG, _>::new(
            calc.as_spec().dex_account(),
            calc.as_spec().oracle(),
            self.querier,
        );

        let out_currency = CalculatorT::OutC::dto().into_super_group();
        super::try_filter_fold_coins(
            calc.as_spec(),
            super::not_out_coins_filter(&out_currency),
            swap_trx,
            |mut trx, coin_in| {
                filtered = true;
                trx.swap_exact_in::<_, _, SwapClient>(&coin_in, &calc.min_output(&coin_in).into())
                    .map(|()| trx)
            },
        )
        .inspect(|_| {
            super::expect_at_lease_one_filtered(filtered, &out_currency);
        })
        .map(Into::into)
    }
}
