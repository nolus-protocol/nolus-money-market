use std::fmt::{Display, Formatter, Result as FmtResult};
use std::marker::PhantomData;

use serde::{Deserialize, Serialize};

use currency::Group;
use finance::{coin::CoinDTO, duration::Duration, instant::Instant};
use platform::batch::Batch;
use sdk::cosmwasm_std::{Binary, Env, QuerierWrapper};

use crate::{
    Connectable, ConnectionParams, Contract, ContractInSwap, Enterable, Error, IBC_TIMEOUT,
    RemoteLeaseTransport as RemoteLeaseTransportT,
    RemoteLeaseTransportFactory as RemoteLeaseTransportFactoryT, Stage, TimeAlarm, error::Result,
};

#[cfg(feature = "migration")]
use super::migration::{_InspectSpec, _MigrateSpec};
use super::{
    SwapTask as SwapTaskT,
    response::{ContinueResult, Handler, Result as HandlerResult},
    timeout,
    transfer_in_finish::TransferInFinish,
};
use cw_time::IntoInstant;

/// Transfer in a coin from DEX
///
#[derive(Serialize, Deserialize)]
pub struct TransferInInit<SwapTask, SEnum, RemoteLeaseTransportFactory>
where
    SwapTask: SwapTaskT,
{
    spec: SwapTask,
    amount_in: CoinDTO<SwapTask::OutG>,
    transport_factory: RemoteLeaseTransportFactory,
    #[serde(skip)]
    _state_enum: PhantomData<SEnum>,
}

impl<SwapTask, SEnum, RemoteLeaseTransportFactory>
    TransferInInit<SwapTask, SEnum, RemoteLeaseTransportFactory>
where
    SwapTask: SwapTaskT,
{
    pub fn new(
        spec: SwapTask,
        amount_in: CoinDTO<SwapTask::OutG>,
        transport_factory: RemoteLeaseTransportFactory,
    ) -> Self {
        Self {
            spec,
            amount_in,
            transport_factory,
            _state_enum: Default::default(),
        }
    }
}

#[cfg(feature = "migration")]
impl<SwapTask, SwapTaskNew, SEnum, SEnumNew, RemoteLeaseTransportFactory>
    _MigrateSpec<SwapTask, SwapTaskNew, SEnumNew>
    for TransferInInit<SwapTask, SEnum, RemoteLeaseTransportFactory>
where
    SwapTask: SwapTaskT,
    SwapTaskNew: SwapTaskT<OutG = SwapTask::OutG>,
{
    type Out = TransferInInit<SwapTaskNew, SEnumNew, RemoteLeaseTransportFactory>;

    fn migrate_spec<MigrateFn>(self, migrate_fn: MigrateFn) -> Self::Out
    where
        MigrateFn: FnOnce(SwapTask) -> SwapTaskNew,
    {
        Self::Out::new(
            migrate_fn(self.spec),
            self.amount_in,
            self.transport_factory,
        )
    }
}

#[cfg(feature = "migration")]
impl<SwapTask, R, SEnum, RemoteLeaseTransportFactory> _InspectSpec<SwapTask, R>
    for TransferInInit<SwapTask, SEnum, RemoteLeaseTransportFactory>
where
    SwapTask: SwapTaskT,
{
    fn inspect_spec<InspectFn>(&self, inspect_fn: InspectFn) -> R
    where
        InspectFn: FnOnce(&SwapTask) -> R,
    {
        inspect_fn(&self.spec)
    }
}

impl<SwapTask, SEnum, RemoteLeaseTransportFactory>
    TransferInInit<SwapTask, SEnum, RemoteLeaseTransportFactory>
where
    SwapTask: SwapTaskT,
    RemoteLeaseTransportFactory:
        RemoteLeaseTransportFactoryT<TopG = <SwapTask::InG as Group>::TopG>,
{
    fn enter_state(&self, now: Instant) -> Result<Batch> {
        let amount = self
            .amount_in
            .into_super_group::<<SwapTask::OutG as Group>::TopG>();
        self.transport_factory
            .transport(&self.spec, now)
            .transfer_back(&amount)
            .map_err(Error::Transport)
    }
}

impl<SwapTask, SEnum, RemoteLeaseTransportFactory>
    TransferInInit<SwapTask, SEnum, RemoteLeaseTransportFactory>
where
    SwapTask: SwapTaskT,
    RemoteLeaseTransportFactory:
        RemoteLeaseTransportFactoryT<TopG = <SwapTask::InG as Group>::TopG>,
    Self: Into<SEnum>,
    TransferInFinish<SwapTask, SEnum, RemoteLeaseTransportFactory>: Into<SEnum>,
{
    fn on_response(self, querier: QuerierWrapper<'_>, env: Env) -> HandlerResult<Self> {
        let finish: TransferInFinish<SwapTask, SEnum, RemoteLeaseTransportFactory> =
            TransferInFinish::new(
                self.spec,
                self.amount_in,
                env.block.time.into_instant() + IBC_TIMEOUT,
                self.transport_factory,
            );
        finish.try_complete(querier, env).map_into()
    }
}

impl<SwapTask, SEnum, RemoteLeaseTransportFactory> Connectable
    for TransferInInit<SwapTask, SEnum, RemoteLeaseTransportFactory>
where
    SwapTask: SwapTaskT,
{
    fn dex(&self) -> &ConnectionParams {
        self.spec.dex_account().dex()
    }
}

impl<SwapTask, SEnum, RemoteLeaseTransportFactory> Enterable
    for TransferInInit<SwapTask, SEnum, RemoteLeaseTransportFactory>
where
    SwapTask: SwapTaskT,
    RemoteLeaseTransportFactory:
        RemoteLeaseTransportFactoryT<TopG = <SwapTask::InG as Group>::TopG>,
{
    fn enter(&self, now: Instant, _querier: QuerierWrapper<'_>) -> Result<Batch> {
        self.enter_state(now)
    }
}

impl<SwapTask, SEnum, RemoteLeaseTransportFactory> Handler
    for TransferInInit<SwapTask, SEnum, RemoteLeaseTransportFactory>
where
    SwapTask: SwapTaskT,
    RemoteLeaseTransportFactory:
        RemoteLeaseTransportFactoryT<TopG = <SwapTask::InG as Group>::TopG>,
    Self: Into<SEnum>,
    TransferInFinish<SwapTask, SEnum, RemoteLeaseTransportFactory>: Into<SEnum>,
{
    type Response = SEnum;
    type SwapResult = SwapTask::Result;

    fn authz_remote_callback(
        &self,
        querier: QuerierWrapper<'_>,
        info: &sdk::cosmwasm_std::MessageInfo,
    ) -> crate::error::Result<()> {
        self.spec.authz_remote_callback(querier, info)
    }

    fn on_response(
        self,
        _data: Binary,
        querier: QuerierWrapper<'_>,
        env: Env,
    ) -> HandlerResult<Self> {
        self.on_response(querier, env)
    }

    fn on_timeout(self, querier: QuerierWrapper<'_>, env: Env) -> ContinueResult<Self> {
        let state_label = self.spec.label();
        timeout::on_timeout_retry(self, state_label, querier, env)
    }
    fn heal(self, querier: QuerierWrapper<'_>, env: Env) -> HandlerResult<Self> {
        self.on_response(querier, env)
    }
}

impl<SwapTask, SEnum, RemoteLeaseTransportFactory> Contract
    for TransferInInit<SwapTask, SEnum, RemoteLeaseTransportFactory>
where
    SwapTask: SwapTaskT + ContractInSwap<StateResponse = <SwapTask as SwapTaskT>::StateResponse>,
{
    type StateResponse = <SwapTask as SwapTaskT>::StateResponse;

    fn state(
        self,
        now: Instant,
        due_projection: Duration,
        querier: QuerierWrapper<'_>,
    ) -> Self::StateResponse {
        self.spec
            .state(Stage::TransferInInit, now, due_projection, querier)
    }
}

impl<SwapTask, ForwardToInnerMsg, RemoteLeaseTransportFactory> Display
    for TransferInInit<SwapTask, ForwardToInnerMsg, RemoteLeaseTransportFactory>
where
    SwapTask: SwapTaskT,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        f.write_fmt(format_args!(
            "TransferInInit at {}",
            self.spec.label().into()
        ))
    }
}

impl<SwapTask, SEnum, RemoteLeaseTransportFactory> TimeAlarm
    for TransferInInit<SwapTask, SEnum, RemoteLeaseTransportFactory>
where
    SwapTask: SwapTaskT,
{
    fn setup_alarm(&self, r#for: Instant) -> Result<Batch> {
        self.spec
            .time_alarm()
            .setup_alarm(r#for)
            .map_err(Into::into)
    }
}
