use currency::{CurrencyDTO, CurrencyDef, Group, MemberOf};
use platform::batch::Batch;
use remote_lease::swap::Builder as SwapBuilder;
use sdk::cosmwasm_std::QuerierWrapper;

use crate::{
    Error, RemoteLeaseTransport as RemoteLeaseTransportT, SlippageCalculator,
    SwapTask as SwapTaskT, WithCalculator, error::Result,
};

pub struct EncodeRequest<'spec, 'querier, SwapTask, RemoteLeaseTransport> {
    spec: &'spec SwapTask,
    transport: RemoteLeaseTransport,
    querier: QuerierWrapper<'querier>,
}

impl<'spec, 'querier, SwapTask, RemoteLeaseTransport>
    EncodeRequest<'spec, 'querier, SwapTask, RemoteLeaseTransport>
{
    pub fn from(
        spec: &'spec SwapTask,
        transport: RemoteLeaseTransport,
        querier: QuerierWrapper<'querier>,
    ) -> Self {
        Self {
            spec,
            transport,
            querier,
        }
    }
}
impl<SwapTask, RemoteLeaseTransport> WithCalculator<SwapTask>
    for EncodeRequest<'_, '_, SwapTask, RemoteLeaseTransport>
where
    SwapTask: SwapTaskT,
    RemoteLeaseTransport: RemoteLeaseTransportT<<SwapTask::InG as Group>::TopG>,
{
    type Output = Result<Batch>;

    fn on<CalculatorT>(self, calc: &CalculatorT) -> Self::Output
    where
        CalculatorT: SlippageCalculator<SwapTask::InG>,
        <<CalculatorT as SlippageCalculator<SwapTask::InG>>::OutC as CurrencyDef>::Group:
            MemberOf<<SwapTask::InG as Group>::TopG> + MemberOf<SwapTask::OutG>,
    {
        let out_currency: CurrencyDTO<<SwapTask::InG as Group>::TopG> =
            CalculatorT::OutC::dto().into_super_group();
        super::try_filter_fold_coins(
            self.spec,
            super::not_out_coins_filter(&out_currency),
            SwapBuilder::new(),
            |builder, coin_in| {
                calc.min_output(&coin_in, self.querier)
                    .and_then(|min_output| {
                        builder
                            .add_coin(coin_in, min_output)
                            .ok_or_else(|| Error::Overflow("calculating minimum swap output"))
                    })
            },
        )
        .and_then(|builder| {
            builder
                .build::<<SwapTask::InG as Group>::TopG>()
                .map_err(Error::BuildSwapRequest)
        })
        .and_then(|swap_params| self.transport.swap(swap_params).map_err(Error::Transport))
    }
}
