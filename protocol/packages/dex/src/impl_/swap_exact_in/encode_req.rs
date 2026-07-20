use std::marker::PhantomData;

use currency::{CurrencyDef, Group, MemberOf};
use platform::batch::Batch;
use sdk::cosmwasm_std::QuerierWrapper;

use crate::{
    RemoteLeaseTransportFactory as RemoteLeaseTransportFactoryT, SlippageCalculator,
    SwapTask as SwapTaskT, WithCalculator, error::Result, impl_::trx::SwapTrx,
};

pub struct EncodeRequest<'spec, 'querier, SwapTask, RemoteLeaseTransportFactory> {
    spec: &'spec SwapTask,
    querier: QuerierWrapper<'querier>,
    _factory: PhantomData<RemoteLeaseTransportFactory>,
}

impl<'spec, 'querier, SwapTask, RemoteLeaseTransportFactory>
    EncodeRequest<'spec, 'querier, SwapTask, RemoteLeaseTransportFactory>
{
    pub fn from(spec: &'spec SwapTask, querier: QuerierWrapper<'querier>) -> Self {
        Self {
            spec,
            querier,
            _factory: PhantomData,
        }
    }
}
impl<SwapTask, RemoteLeaseTransportFactory> WithCalculator<SwapTask>
    for EncodeRequest<'_, '_, SwapTask, RemoteLeaseTransportFactory>
where
    SwapTask: SwapTaskT,
    RemoteLeaseTransportFactory: RemoteLeaseTransportFactoryT,
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
                        trx.swap_exact_in::<_, _, RemoteLeaseTransportFactory::Transport<'_>>(
                            &coin_in,
                            &min_output.into(),
                        )
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
