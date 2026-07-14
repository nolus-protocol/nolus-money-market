use std::{
    fmt::{Display, Formatter, Result as FmtResult},
    marker::PhantomData,
};

use serde::{Deserialize, Serialize};

use finance::duration::Duration;
use finance::instant::Instant;
use platform::{
    batch::{Batch, Emitter},
    ica::ErrorResponse as ICAErrorResponse,
    message::Response as MessageResponse,
};
use sdk::cosmwasm_std::{Binary, Env, QuerierWrapper};

use crate::{
    CoinsNb, Contract, ContractInSwap, Enterable, Stage, SwapTask as SwapTaskT, TimeAlarm,
    TransportOut as TransportOutT, TransportOutFactory as TransportOutFactoryT, error::Result,
    swap::ExactAmountIn,
};

#[cfg(feature = "migration")]
use super::migration::{_InspectSpec, _MigrateSpec};
use super::{
    response::{self, ContinueResult, Handler, Result as HandlerResult},
    swap_exact_in::SwapExactIn,
    timeout,
};
use cw_time::IntoInstant;

/// Transfer out a list of coins to DEX
///
/// Supports up to `CoinsNb::MAX` number of coins.
/// In does it in a single transaction with multiple messages expecting
/// an acknowledgment per message.
#[derive(Serialize, Deserialize)]
#[cfg_attr(test, derive(Clone, Debug, PartialEq, Eq))]
#[serde(
    bound(
        serialize = "SwapTask: Serialize,
                        TransportOutFactory: Serialize",
        deserialize = "SwapTask: Deserialize<'de> + SwapTaskT,
                        TransportOutFactory: Deserialize<'de>"
    ),
    deny_unknown_fields,
    rename_all = "snake_case"
)]
pub struct TransferOut<SwapTask, SEnum, TransportOutFactory, SwapClient> {
    spec: SwapTask,
    transport_out_fry: TransportOutFactory,
    acks_left: CoinsNb,
    #[serde(skip)]
    _state_enum: PhantomData<SEnum>,
    #[serde(skip)]
    _swap_client: PhantomData<SwapClient>,
}

impl<SwapTask, SEnum, TransportOutFactory, SwapClient>
    TransferOut<SwapTask, SEnum, TransportOutFactory, SwapClient>
where
    SwapTask: SwapTaskT,
    TransportOutFactory: TransportOutFactoryT,
{
    pub fn new(spec: SwapTask, transport: TransportOutFactory) -> Self {
        let acks_left = Self::coins_len(&spec);
        Self::internal_new(spec, transport, acks_left)
    }

    fn internal_new(
        spec: SwapTask,
        transport_out_fry: TransportOutFactory,
        acks_left: CoinsNb,
    ) -> Self {
        let ret = Self {
            spec,
            transport_out_fry,
            acks_left,
            _state_enum: PhantomData,
            _swap_client: PhantomData,
        };
        debug_assert!(ret.invariant());
        ret
    }

    fn invariant(&self) -> bool {
        let coins_nb = Self::coins_len(&self.spec);
        0 < self.acks_left && self.acks_left <= coins_nb
    }

    fn coins_len(spec: &SwapTask) -> CoinsNb {
        let ret = spec.coins().into_iter().count();
        assert!(ret > 0, "The swap task did not provide any coins!");
        ret.try_into()
            .expect("Functionality doesn't support this many coins!")
    }

    fn generate_requests(&self, now: Instant) -> Batch {
        debug_assert_eq!(
            Self::coins_len(&self.spec),
            self.acks_left,
            "calling 'enter_state' past initialization"
        );

        let mut transport = self.transport_out_fry.transport(&self.spec, now);

        self.spec
            .coins()
            .into_iter()
            .for_each(|coin| transport.send(&coin));
        transport.into()
    }

    fn next(self) -> Self {
        debug_assert!(!self.last_ack());

        let acks_left = self.acks_left.checked_sub(1).expect(
            "the method contract precondition `!self.last_ack()` should have been respected",
        );

        Self::internal_new(self.spec, self.transport_out_fry, acks_left)
    }

    fn last_ack(&self) -> bool {
        debug_assert!(self.acks_left > 0);
        self.acks_left == 1
    }
}

impl<SwapTask, SEnum, TransportOutFactory, SwapClient>
    TransferOut<SwapTask, SEnum, TransportOutFactory, SwapClient>
where
    SwapTask: SwapTaskT,
    TransportOutFactory: TransportOutFactoryT,
    SwapClient: ExactAmountIn,
    Self: Into<SEnum>,
    SwapExactIn<SwapTask, SEnum, SwapClient>: Into<SEnum>,
{
    fn on_response<NextState, Label>(
        next: NextState,
        label: Label,
        msgs: Batch,
    ) -> ContinueResult<Self>
    where
        NextState: Enterable + Into<SEnum>,
        Label: Into<String>,
    {
        let emitter = Emitter::of_type(label);
        response::res_continue::<_, _, Self>(
            MessageResponse::messages_with_event(msgs, emitter),
            next,
        )
    }
}

impl<SwapTask, SEnum, TransportOutFactory, SwapClient> Enterable
    for TransferOut<SwapTask, SEnum, TransportOutFactory, SwapClient>
where
    SwapTask: SwapTaskT,
    TransportOutFactory: TransportOutFactoryT,
    SwapClient: ExactAmountIn,
    Self: Into<SEnum>,
    SwapExactIn<SwapTask, SEnum, SwapClient>: Into<SEnum>,
{
    fn enter(&self, now: Instant, _querier: QuerierWrapper<'_>) -> Result<Batch> {
        Ok(self.generate_requests(now))
    }
}

impl<SwapTask, SEnum, TransportOutFactory, SwapClient> Handler
    for TransferOut<SwapTask, SEnum, TransportOutFactory, SwapClient>
where
    SwapTask: SwapTaskT,
    TransportOutFactory: TransportOutFactoryT,
    SwapClient: ExactAmountIn,
    Self: Into<SEnum>,
    SwapExactIn<SwapTask, SEnum, SwapClient>: Into<SEnum>,
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
        _resp: Binary,
        querier: QuerierWrapper<'_>,
        env: Env,
    ) -> HandlerResult<Self> {
        let label = self.spec.label();
        if self.last_ack() {
            let next = SwapExactIn::new(self.spec);
            next.enter(env.block.time.into_instant(), querier)
                .and_then(|msgs| Self::on_response(next, label, msgs))
        } else {
            Self::on_response(self.next(), label, Batch::default())
        }
        .into()
    }

    fn on_timeout(self, querier: QuerierWrapper<'_>, env: Env) -> ContinueResult<Self> {
        let state_label = self.spec.label();
        timeout::on_timeout_retry(self, state_label, querier, env)
    }

    // occasionslly, we get errors from handling the transfer receive message at the remote network
    // we cannot do anything else except keep trying to transfer again
    fn on_error(
        self,
        _response: ICAErrorResponse,
        querier: QuerierWrapper<'_>,
        env: Env,
    ) -> HandlerResult<Self> {
        self.on_timeout(querier, env).into()
    }
}

impl<SwapTask, SEnum, TransportOutFactory, SwapClient> Contract
    for TransferOut<SwapTask, SEnum, TransportOutFactory, SwapClient>
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
            .state(Stage::TransferOut, now, due_projection, querier)
    }
}

impl<SwapTask, SEnum, TransportOutFactory, SwapClient> Display
    for TransferOut<SwapTask, SEnum, TransportOutFactory, SwapClient>
where
    SwapTask: SwapTaskT,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        f.write_fmt(format_args!("TransferOut at {}", self.spec.label().into()))
    }
}

impl<SwapTask, SEnum, TransportOutFactory, SwapClient> TimeAlarm
    for TransferOut<SwapTask, SEnum, TransportOutFactory, SwapClient>
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

#[cfg(feature = "migration")]
impl<SwapTask, SwapTaskNew, SEnum, SEnumNew, TransportOutFactory, SwapClient>
    _MigrateSpec<SwapTask, SwapTaskNew, SEnumNew>
    for TransferOut<SwapTask, SEnum, TransportOutFactory, SwapClient>
where
    Self: Sized,
    SwapTaskNew: SwapTaskT,
    TransportOutFactory: TransportOutFactoryT,
    TransferOut<SwapTaskNew, SEnumNew, TransportOutFactory, SwapClient>: Into<SEnumNew>,
{
    type Out = TransferOut<SwapTaskNew, SEnumNew, TransportOutFactory, SwapClient>;

    fn migrate_spec<MigrateFn>(self, migrate_fn: MigrateFn) -> Self::Out
    where
        MigrateFn: FnOnce(SwapTask) -> SwapTaskNew,
    {
        Self::Out::new(migrate_fn(self.spec), self.transport_out_fry)
    }
}

#[cfg(feature = "migration")]
impl<SwapTask, R, SEnum, TransportOutFactory, SwapClient> _InspectSpec<SwapTask, R>
    for TransferOut<SwapTask, SEnum, TransportOutFactory, SwapClient>
{
    fn inspect_spec<InspectFn>(&self, inspect_fn: InspectFn) -> R
    where
        InspectFn: FnOnce(&SwapTask) -> R,
    {
        inspect_fn(&self.spec)
    }
}
