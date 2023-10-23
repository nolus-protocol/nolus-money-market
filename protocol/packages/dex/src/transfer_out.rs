use std::{
    fmt::{Display, Formatter, Result as FmtResult},
    marker::PhantomData,
    result::Result as StdResult,
};

use serde::{Deserialize, Serialize};

use currency::Group;
use finance::{coin::CoinDTO, zero::Zero};
use platform::{
    batch::{Batch, Emitter},
    message::Response as MessageResponse,
    never::{self, Never},
};
use sdk::cosmwasm_std::{Binary, Deps, Env, QuerierWrapper, Timestamp};

use crate::{
    error::{Error, Result},
    ica_connector::Enterable,
    response::{self, ContinueResult, Handler, Result as HandlerResult},
    swap_task::{CoinVisitor, IterNext},
    timeout,
    trx::TransferOutTrx,
    Contract, ContractInSwap, TimeAlarm, TransferOutState,
};
#[cfg(feature = "migration")]
use crate::{InspectSpec, MigrateSpec};

use super::{
    coin_index,
    swap_exact_in::SwapExactIn,
    swap_task::{CoinsNb, IterState, SwapTask as SwapTaskT},
};

/// Transfer out a list of coins to DEX
///
/// Supports up to `CoinsNb::MAX` number of coins.
#[derive(Serialize, Deserialize)]
pub struct TransferOut<SwapTask, SEnum> {
    spec: SwapTask,
    coin_index: CoinsNb,
    last_coin_index: CoinsNb,
    #[serde(skip)]
    _state_enum: PhantomData<SEnum>,
}

impl<SwapTask, SEnum> TransferOut<SwapTask, SEnum>
where
    SwapTask: SwapTaskT,
    Self: Into<SEnum>,
    SwapExactIn<SwapTask, SEnum>: Into<SEnum>,
{
    fn next(self) -> Self {
        debug_assert!(!self.last_coin());

        debug_assert!(self.coin_index < CoinsNb::MAX); // though already checked implicitly with the `self.last_coin()`
        let next_index = self.coin_index + 1;

        Self::new_with_index(self.spec, next_index, self.last_coin_index)
    }

    fn last_coin(&self) -> bool {
        debug_assert!(self.coin_index <= self.last_coin_index);
        self.coin_index == self.last_coin_index
    }

    fn enter_state(&self, now: Timestamp, _querier: &QuerierWrapper<'_>) -> Result<Batch> {
        struct SendWorker<'a>(TransferOutTrx<'a>, bool);
        impl<'a> CoinVisitor for SendWorker<'a> {
            type Result = ();
            type Error = Error;

            fn visit<G>(&mut self, coin: &CoinDTO<G>) -> Result<Self::Result>
            where
                G: Group,
            {
                debug_assert!(!self.1, "already visited");
                self.1 = true;
                self.0.send(coin)
            }
        }

        let mut sender = SendWorker(self.spec.dex_account().transfer_to(now), false);
        let iter_state = coin_index::visit_at_index(&self.spec, self.coin_index, &mut sender)?;
        debug_assert!(sender.1, "the coin index is invalid");
        debug_assert_eq!(iter_state == IterState::Complete, self.last_coin());
        Ok(sender.0.into())
    }

    fn on_response<NextState, Label>(
        next: NextState,
        label: Label,
        now: Timestamp,
        querier: &QuerierWrapper<'_>,
    ) -> ContinueResult<Self>
    where
        NextState: Enterable + Into<SEnum>,
        Label: Into<String>,
    {
        next.enter(now, querier).and_then(|batch| {
            let emitter = Emitter::of_type(label);
            response::res_continue::<_, _, Self>(
                MessageResponse::messages_with_events(batch, emitter),
                next,
            )
        })
    }
}

impl<SwapTask, SEnum> TransferOut<SwapTask, SEnum>
where
    SwapTask: SwapTaskT,
{
    pub fn new(spec: SwapTask) -> Self {
        let first_index = Default::default();
        let last_coin_index = Self::last_coin_index(&spec);
        Self::new_with_index(spec, first_index, last_coin_index)
    }

    fn new_with_index(spec: SwapTask, coin_index: CoinsNb, last_coin_index: CoinsNb) -> Self {
        debug_assert!(coin_index <= last_coin_index);
        Self {
            spec,
            coin_index,
            last_coin_index,
            _state_enum: PhantomData,
        }
    }

    fn last_coin_index(spec: &SwapTask) -> CoinsNb {
        let mut counter = Counter::default();
        let _res = never::safe_unwrap(spec.on_coins(&mut counter));

        #[cfg(debug_assertions)]
        debug_assert_eq!(_res, IterState::Complete);
        counter.last_index()
    }
}

impl<SwapTask, SEnum> Enterable for TransferOut<SwapTask, SEnum>
where
    SwapTask: SwapTaskT,
    Self: Into<SEnum>,
    SwapExactIn<SwapTask, SEnum>: Into<SEnum>,
{
    fn enter(&self, now: Timestamp, querier: &QuerierWrapper<'_>) -> Result<Batch> {
        self.enter_state(now, querier)
    }
}

impl<SwapTask, SEnum> Handler for TransferOut<SwapTask, SEnum>
where
    SwapTask: SwapTaskT,
    Self: Into<SEnum>,
    SwapExactIn<SwapTask, SEnum>: Into<SEnum>,
{
    type Response = SEnum;
    type SwapResult = SwapTask::Result;

    fn on_response(self, _resp: Binary, deps: Deps<'_>, env: Env) -> HandlerResult<Self> {
        let label = self.spec.label();
        let now = env.block.time;
        if self.last_coin() {
            Self::on_response(SwapExactIn::new(self.spec), label, now, &deps.querier)
        } else {
            Self::on_response(self.next(), label, now, &deps.querier)
        }
        .into()
    }

    fn on_timeout(self, deps: Deps<'_>, env: Env) -> ContinueResult<Self> {
        let state_label = self.spec.label();
        timeout::on_timeout_retry(self, state_label, deps, env)
    }
}

impl<SwapTask, SEnum> Contract for TransferOut<SwapTask, SEnum>
where
    SwapTask: ContractInSwap<TransferOutState, <SwapTask as SwapTaskT>::StateResponse> + SwapTaskT,
{
    type StateResponse = <SwapTask as SwapTaskT>::StateResponse;

    fn state(self, now: Timestamp, querier: &QuerierWrapper<'_>) -> Self::StateResponse {
        self.spec.state(now, querier)
    }
}

impl<SwapTask, SEnum> Display for TransferOut<SwapTask, SEnum>
where
    SwapTask: SwapTaskT,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        f.write_fmt(format_args!("TransferOut at {}", self.spec.label().into()))
    }
}

impl<SwapTask, SEnum> TimeAlarm for TransferOut<SwapTask, SEnum>
where
    SwapTask: SwapTaskT,
{
    fn setup_alarm(&self, forr: Timestamp) -> Result<Batch> {
        self.spec.time_alarm().setup_alarm(forr).map_err(Into::into)
    }
}

#[cfg(feature = "migration")]
impl<SwapTask, SwapTaskNew, SEnum, SEnumNew> MigrateSpec<SwapTask, SwapTaskNew, SEnumNew>
    for TransferOut<SwapTask, SEnum>
where
    Self: Sized,
    SwapTaskNew: SwapTaskT,
    TransferOut<SwapTaskNew, SEnumNew>: Into<SEnumNew>,
{
    type Out = TransferOut<SwapTaskNew, SEnumNew>;

    fn migrate_spec<MigrateFn>(self, migrate_fn: MigrateFn) -> Self::Out
    where
        MigrateFn: FnOnce(SwapTask) -> SwapTaskNew,
    {
        Self::Out::new_with_index(migrate_fn(self.spec), self.coin_index, self.last_coin_index)
    }
}

#[cfg(feature = "migration")]
impl<SwapTask, R, SEnum> InspectSpec<SwapTask, R> for TransferOut<SwapTask, SEnum> {
    fn inspect_spec<InspectFn>(&self, inspect_fn: InspectFn) -> R
    where
        InspectFn: FnOnce(&SwapTask) -> R,
    {
        inspect_fn(&self.spec)
    }
}

#[derive(Default)]
struct Counter(Option<CoinsNb>);
impl Counter {
    fn last_index(&self) -> CoinsNb {
        self.0.expect("The swap task did not provide any coins")
    }
}
impl CoinVisitor for Counter {
    type Result = IterNext;
    type Error = Never;

    fn visit<G>(&mut self, _coin: &CoinDTO<G>) -> StdResult<Self::Result, Self::Error>
    where
        G: Group,
    {
        let next_idx = self.0.map_or(CoinsNb::ZERO, |prev_idx| {
            prev_idx
                .checked_add(1)
                .expect("The swap task exceeds the max number of coins `CoinsNb::MAX`")
        });
        self.0 = Some(next_idx);
        Ok(IterNext::Continue)
    }
}

#[cfg(test)]
mod test {
    use currency::test::{SuperGroup, SuperGroupTestC1};
    use finance::coin::{Coin, CoinDTO};

    use crate::swap_task::{CoinVisitor, CoinsNb, IterNext};

    use super::Counter;

    fn coin() -> CoinDTO<SuperGroup> {
        Coin::<SuperGroupTestC1>::new(22).into()
    }

    #[test]
    fn index_zero() {
        let mut c = Counter::default();
        let r = c.visit::<SuperGroup>(&coin()).unwrap();
        assert_eq!(r, IterNext::Continue);
        assert_eq!(c.last_index(), 0);
    }

    #[test]
    fn index_one() {
        let mut c = Counter::default();
        let r = c.visit::<SuperGroup>(&coin()).unwrap();
        assert_eq!(r, IterNext::Continue);
        let r = c.visit::<SuperGroup>(&coin()).unwrap();
        assert_eq!(r, IterNext::Continue);
        assert_eq!(c.last_index(), 1);
    }

    #[test]
    fn index_max() {
        let mut c = Counter::default();
        for _i in 0..=CoinsNb::MAX {
            let r = c.visit::<SuperGroup>(&coin()).unwrap();
            assert_eq!(r, IterNext::Continue);
        }
        assert_eq!(c.last_index(), CoinsNb::MAX);
    }
}
