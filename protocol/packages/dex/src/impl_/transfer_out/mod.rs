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
    error::Result,
};

#[cfg(feature = "migration")]
use super::migration::{InspectSpec, MigrateSpec};
use super::{
    next_leg::NextLeg as NextLegT,
    response::{self, ContinueResult, Handler, Result as HandlerResult},
    swap_exact_in::SwapExactIn,
    timeout,
    trx::TransferOutTrx,
};

/// Transfer out a list of coins to DEX
///
/// Supports up to `CoinsNb::MAX` number of coins.
/// In does it in a single transaction with multiple messages expecting
/// an acknowledgment per message.
///
/// Once all transfers get acknowledged the workflow proceeds with
/// `NextLeg`, defaulting to the local DEX swap.
#[derive(Serialize, Deserialize)]
#[cfg_attr(test, derive(Clone, Debug, PartialEq, Eq))]
#[serde(
    bound(
        serialize = "SwapTask: Serialize",
        deserialize = "SwapTask: Deserialize<'de> + SwapTaskT"
    ),
    deny_unknown_fields,
    rename_all = "snake_case"
)]
pub struct TransferOut<
    SwapTask,
    SEnum,
    SwapClient,
    NextLeg = SwapExactIn<SwapTask, SEnum, SwapClient>,
> {
    spec: SwapTask,
    acks_left: CoinsNb,
    #[serde(skip)]
    _state_enum: PhantomData<SEnum>,
    #[serde(skip)]
    _swap_client: PhantomData<SwapClient>,
    #[serde(skip)]
    _next_leg: PhantomData<NextLeg>,
}

impl<SwapTask, SEnum, SwapClient, NextLeg> TransferOut<SwapTask, SEnum, SwapClient, NextLeg>
where
    SwapTask: SwapTaskT,
{
    pub fn new(spec: SwapTask) -> Self {
        let acks_left = Self::coins_len(&spec);
        Self::internal_new(spec, acks_left)
    }

    fn internal_new(spec: SwapTask, acks_left: CoinsNb) -> Self {
        let ret = Self {
            spec,
            acks_left,
            _state_enum: PhantomData,
            _swap_client: PhantomData,
            _next_leg: PhantomData,
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
        let mut trx = TransferOutTrx::new(self.spec.dex_account(), now);

        self.spec
            .coins()
            .into_iter()
            .for_each(|coin| trx.send(&coin));
        trx.into()
    }

    fn next(self) -> Self {
        debug_assert!(!self.last_ack());

        let acks_left = self.acks_left.checked_sub(1).expect(
            "the method contract precondition `!self.last_ack()` should have been respected",
        );

        Self::internal_new(self.spec, acks_left)
    }

    fn last_ack(&self) -> bool {
        debug_assert!(self.acks_left > 0);
        self.acks_left == 1
    }
}

impl<SwapTask, SEnum, SwapClient, NextLeg> Enterable
    for TransferOut<SwapTask, SEnum, SwapClient, NextLeg>
where
    SwapTask: SwapTaskT,
{
    fn enter(&self, now: Instant, _querier: QuerierWrapper<'_>) -> Result<Batch> {
        Ok(self.generate_requests(now))
    }
}

impl<SwapTask, SEnum, SwapClient, NextLeg> Handler
    for TransferOut<SwapTask, SEnum, SwapClient, NextLeg>
where
    SwapTask: SwapTaskT,
    Self: Into<SEnum>,
    NextLeg: NextLegT<SwapTask> + Handler<Response = SEnum, SwapResult = SwapTask::Result>,
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
        if self.last_ack() {
            NextLeg::enter_from(self.spec, querier, &env).map_into()
        } else {
            let label = self.spec.label();
            let next = self.next();
            response::res_continue::<_, _, Self>(
                MessageResponse::messages_with_event(Batch::default(), Emitter::of_type(label)),
                next,
            )
            .into()
        }
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

impl<SwapTask, SEnum, SwapClient, NextLeg> Contract
    for TransferOut<SwapTask, SEnum, SwapClient, NextLeg>
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

impl<SwapTask, SEnum, SwapClient, NextLeg> Display
    for TransferOut<SwapTask, SEnum, SwapClient, NextLeg>
where
    SwapTask: SwapTaskT,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        f.write_fmt(format_args!("TransferOut at {}", self.spec.label().into()))
    }
}

impl<SwapTask, SEnum, SwapClient, NextLeg> TimeAlarm
    for TransferOut<SwapTask, SEnum, SwapClient, NextLeg>
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
impl<SwapTask, SwapTaskNew, SEnum, SEnumNew, SwapClient, NextLeg>
    MigrateSpec<SwapTask, SwapTaskNew, SEnumNew>
    for TransferOut<SwapTask, SEnum, SwapClient, NextLeg>
where
    Self: Sized,
    SwapTaskNew: SwapTaskT,
    NextLeg: MigrateSpec<SwapTask, SwapTaskNew, SEnumNew>,
    TransferOut<SwapTaskNew, SEnumNew, SwapClient, NextLeg::Out>: Into<SEnumNew>,
{
    type Out = TransferOut<SwapTaskNew, SEnumNew, SwapClient, NextLeg::Out>;

    fn migrate_spec<MigrateFn>(self, migrate_fn: MigrateFn) -> Self::Out
    where
        MigrateFn: FnOnce(SwapTask) -> SwapTaskNew,
    {
        Self::Out::new(migrate_fn(self.spec))
    }
}

#[cfg(feature = "migration")]
impl<SwapTask, R, SEnum, SwapClient, NextLeg> InspectSpec<SwapTask, R>
    for TransferOut<SwapTask, SEnum, SwapClient, NextLeg>
{
    fn inspect_spec<InspectFn>(&self, inspect_fn: InspectFn) -> R
    where
        InspectFn: FnOnce(&SwapTask) -> R,
    {
        inspect_fn(&self.spec)
    }
}

#[cfg(test)]
mod tests {
    use currency::test::{SuperGroupTestC1, SuperGroupTestC2};
    use finance::coin::Coin;

    use crate::impl_::remote_swap::mock::MockSpec;

    use super::TransferOut;

    type TestTransferOut = TransferOut<MockSpec, (), ()>;

    #[test]
    fn serialization_shape_unchanged() {
        let spec_json =
            sdk::cosmwasm_std::to_json_string(&spec()).expect("the spec should serialize");
        assert_eq!(
            format!(r#"{{"spec":{spec_json},"acks_left":2}}"#),
            sdk::cosmwasm_std::to_json_string(&TestTransferOut::new(spec()))
                .expect("the state should serialize")
        );
    }

    #[test]
    fn serde_round_trips() {
        let transfer_out = TestTransferOut::new(spec());
        let restored: TestTransferOut = sdk::cosmwasm_std::to_json_vec(&transfer_out)
            .and_then(sdk::cosmwasm_std::from_json)
            .expect("the state should round-trip");
        assert_eq!(transfer_out.spec, restored.spec);
        assert_eq!(transfer_out.acks_left, restored.acks_left);
    }

    fn spec() -> MockSpec {
        MockSpec::new(vec![
            Coin::<SuperGroupTestC2>::new(100).into(),
            Coin::<SuperGroupTestC1>::new(50).into(),
        ])
    }
}
