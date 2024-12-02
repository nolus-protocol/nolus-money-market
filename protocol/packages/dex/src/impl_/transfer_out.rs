use std::{
    fmt::{Display, Formatter, Result as FmtResult},
    marker::PhantomData,
    result::Result as StdResult,
};

use serde::{Deserialize, Serialize};

use currency::{Group, MemberOf};
use finance::{coin::CoinDTO, zero::Zero};
use platform::{
    batch::{Batch, Emitter},
    message::Response as MessageResponse,
};
use sdk::cosmwasm_std::{Binary, Env, QuerierWrapper, Timestamp};

use crate::{
    error::{Error, Result},
    swap::ExactAmountIn,
};
#[cfg(feature = "migration")]
use crate::{InspectSpec, MigrateSpec};

use super::{
    coin_index,
    ica_connector::Enterable,
    response::{self, ContinueResult, Handler, Result as HandlerResult},
    swap_exact_in::SwapExactIn,
    swap_task::{CoinVisitor, CoinsNb, IterNext, IterState, SwapTask as SwapTaskT},
    timeout,
    trx::TransferOutTrx,
    Contract, ContractInSwap, TimeAlarm, TransferOutState,
};

/// Transfer out a list of coins to DEX
///
/// Supports up to `CoinsNb::MAX` number of coins.
#[derive(Serialize, Deserialize)]
#[serde(bound(
    serialize = "SwapTask: Serialize",
    deserialize = "SwapTask: Deserialize<'de>",
))]
pub struct TransferOut<SwapTask, SEnum, SwapGroup, SwapClient> {
    spec: SwapTask,
    coin_index: CoinsNb,
    last_coin_index: CoinsNb,
    #[serde(skip)]
    _state_enum: PhantomData<SEnum>,
    #[serde(skip)]
    _swap_group: PhantomData<SwapGroup>,
    #[serde(skip)]
    _swap_client: PhantomData<SwapClient>,
}

impl<SwapTask, SEnum, SwapGroup, SwapClient> TransferOut<SwapTask, SEnum, SwapGroup, SwapClient>
where
    SwapTask: SwapTaskT,
    SwapGroup: Group,
    SwapClient: ExactAmountIn,
    Self: Into<SEnum>,
    SwapExactIn<SwapTask, SEnum, SwapGroup, SwapClient>: Into<SEnum>,
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

    fn enter_state(&self, now: Timestamp, _querier: QuerierWrapper<'_>) -> Result<Batch> {
        struct SendWorker<'a, G> {
            trx: TransferOutTrx<'a>,
            sent: bool,
            _group: PhantomData<G>,
        }

        impl<GIn> CoinVisitor for SendWorker<'_, GIn>
        where
            GIn: Group,
        {
            type GIn = GIn;

            type Result = ();

            type Error = Error;

            fn visit<G>(&mut self, coin: &CoinDTO<G>) -> Result<Self::Result>
            where
                G: Group + MemberOf<Self::GIn>,
            {
                debug_assert!(!self.sent, "already visited");
                self.sent = true;
                self.trx.send(coin)
            }
        }

        let mut sender = SendWorker {
            trx: self.spec.dex_account().transfer_to(now),
            sent: false,
            _group: PhantomData::<SwapTask::InG>,
        };
        let iter_state = coin_index::visit_at_index(&self.spec, self.coin_index, &mut sender)?;
        debug_assert!(sender.sent, "the coin index is invalid");
        debug_assert_eq!(iter_state == IterState::Complete, self.last_coin());
        Ok(sender.trx.into())
    }

    fn on_response<NextState, Label>(
        next: NextState,
        label: Label,
        now: Timestamp,
        querier: QuerierWrapper<'_>,
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

impl<SwapTask, SEnum, SwapGroup, SwapClient> TransferOut<SwapTask, SEnum, SwapGroup, SwapClient>
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
            _swap_group: PhantomData,
            _swap_client: PhantomData,
        }
    }

    fn last_coin_index(spec: &SwapTask) -> CoinsNb {
        let mut counter = Counter::<SwapTask::InG>::default();

        match spec.on_coins(&mut counter) {
            #[cfg_attr(not(debug_assertions), expect(unused_variables))]
            Ok(iter_state) => {
                #[cfg(debug_assertions)]
                assert_eq!(iter_state, IterState::Complete);
            }
            Err(TooManyCoins {}) => {
                unimplemented!("Functionality doesn't support this many coins!");
            }
        }

        counter.last_index()
    }
}

impl<SwapTask, SEnum, SwapGroup, SwapClient> Enterable
    for TransferOut<SwapTask, SEnum, SwapGroup, SwapClient>
where
    SwapTask: SwapTaskT,
    SwapGroup: Group,
    SwapClient: ExactAmountIn,
    Self: Into<SEnum>,
    SwapExactIn<SwapTask, SEnum, SwapGroup, SwapClient>: Into<SEnum>,
{
    fn enter(&self, now: Timestamp, querier: QuerierWrapper<'_>) -> Result<Batch> {
        self.enter_state(now, querier)
    }
}

impl<SwapTask, SEnum, SwapGroup, SwapClient> Handler
    for TransferOut<SwapTask, SEnum, SwapGroup, SwapClient>
where
    SwapTask: SwapTaskT,
    SwapGroup: Group,
    SwapClient: ExactAmountIn,
    Self: Into<SEnum>,
    SwapExactIn<SwapTask, SEnum, SwapGroup, SwapClient>: Into<SEnum>,
{
    type Response = SEnum;
    type SwapResult = SwapTask::Result;

    fn on_response(
        self,
        _resp: Binary,
        querier: QuerierWrapper<'_>,
        env: Env,
    ) -> HandlerResult<Self> {
        let label = self.spec.label();
        let now = env.block.time;
        if self.last_coin() {
            Self::on_response(SwapExactIn::new(self.spec), label, now, querier)
        } else {
            Self::on_response(self.next(), label, now, querier)
        }
        .into()
    }

    fn on_timeout(self, querier: QuerierWrapper<'_>, env: Env) -> ContinueResult<Self> {
        let state_label = self.spec.label();
        timeout::on_timeout_retry(self, state_label, querier, env)
    }

    // occasionslly, we get errors from handling the transfer receive message at the remote network
    // we cannot do anything else except keep trying to transfer again
    fn on_error(self, querier: QuerierWrapper<'_>, env: Env) -> ContinueResult<Self> {
        self.on_timeout(querier, env)
    }
}

impl<SwapTask, SEnum, SwapGroup, SwapClient> Contract
    for TransferOut<SwapTask, SEnum, SwapGroup, SwapClient>
where
    SwapTask: SwapTaskT
        + ContractInSwap<TransferOutState, StateResponse = <SwapTask as SwapTaskT>::StateResponse>,
{
    type StateResponse = <SwapTask as SwapTaskT>::StateResponse;

    fn state(self, now: Timestamp, querier: QuerierWrapper<'_>) -> Self::StateResponse {
        self.spec.state(now, querier)
    }
}

impl<SwapTask, SEnum, SwapGroup, SwapClient> Display
    for TransferOut<SwapTask, SEnum, SwapGroup, SwapClient>
where
    SwapTask: SwapTaskT,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        f.write_fmt(format_args!("TransferOut at {}", self.spec.label().into()))
    }
}

impl<SwapTask, SEnum, SwapGroup, SwapClient> TimeAlarm
    for TransferOut<SwapTask, SEnum, SwapGroup, SwapClient>
where
    SwapTask: SwapTaskT,
{
    fn setup_alarm(&self, r#for: Timestamp) -> Result<Batch> {
        self.spec
            .time_alarm()
            .setup_alarm(r#for)
            .map_err(Into::into)
    }
}

#[cfg(feature = "migration")]
impl<SwapTask, SwapTaskNew, SEnum, SEnumNew, SwapGroup, SwapClient>
    MigrateSpec<SwapTask, SwapTaskNew, SEnumNew>
    for TransferOut<SwapTask, SEnum, SwapGroup, SwapClient>
where
    Self: Sized,
    SwapTaskNew: SwapTaskT,
    TransferOut<SwapTaskNew, SEnumNew, SwapGroup, SwapClient>: Into<SEnumNew>,
{
    type Out = TransferOut<SwapTaskNew, SEnumNew, SwapGroup, SwapClient>;

    fn migrate_spec<MigrateFn>(self, migrate_fn: MigrateFn) -> Self::Out
    where
        MigrateFn: FnOnce(SwapTask) -> SwapTaskNew,
    {
        Self::Out::new_with_index(migrate_fn(self.spec), self.coin_index, self.last_coin_index)
    }
}

#[cfg(feature = "migration")]
impl<SwapTask, R, SEnum, SwapGroup, SwapClient> InspectSpec<SwapTask, R>
    for TransferOut<SwapTask, SEnum, SwapGroup, SwapClient>
{
    fn inspect_spec<InspectFn>(&self, inspect_fn: InspectFn) -> R
    where
        InspectFn: FnOnce(&SwapTask) -> R,
    {
        inspect_fn(&self.spec)
    }
}

struct Counter<G> {
    last_index: Option<CoinsNb>,
    _group: PhantomData<G>,
}

impl<G> Counter<G> {
    fn last_index(&self) -> CoinsNb {
        self.last_index
            .expect("The swap task did not provide any coins!")
    }
}

impl<GIn> CoinVisitor for Counter<GIn>
where
    GIn: Group,
{
    type GIn = GIn;

    type Result = IterNext;

    type Error = TooManyCoins;

    fn visit<G>(&mut self, _coin: &CoinDTO<G>) -> StdResult<Self::Result, Self::Error>
    where
        G: Group + MemberOf<Self::GIn>,
    {
        self.last_index = Some(
            self.last_index
                .map(|last_index| last_index.checked_add(1).ok_or(const { TooManyCoins {} }))
                .transpose()?
                .unwrap_or(const { CoinsNb::ZERO }),
        );

        const { Ok(IterNext::Continue) }
    }
}

impl<G> Default for Counter<G>
where
    G: Group,
{
    fn default() -> Self {
        Self {
            last_index: Default::default(),
            _group: Default::default(),
        }
    }
}

#[derive(Debug)]
struct TooManyCoins;

#[cfg(test)]
mod test {
    use currency::test::{SuperGroup, SuperGroupTestC1};
    use finance::coin::{Coin, CoinDTO};

    use crate::impl_::swap_task::{CoinVisitor, CoinsNb, IterNext};

    use super::Counter;

    fn coin() -> CoinDTO<SuperGroup> {
        Coin::<SuperGroupTestC1>::new(22).into()
    }

    #[test]
    fn index_zero() {
        let mut c = Counter::<SuperGroup>::default();
        let r = c.visit(&coin()).unwrap();
        assert_eq!(r, IterNext::Continue);
        assert_eq!(c.last_index(), 0);
    }

    #[test]
    fn index_one() {
        let mut c = Counter::<SuperGroup>::default();
        let r = c.visit(&coin()).unwrap();
        assert_eq!(r, IterNext::Continue);
        let r = c.visit(&coin()).unwrap();
        assert_eq!(r, IterNext::Continue);
        assert_eq!(c.last_index(), 1);
    }

    #[test]
    fn index_max() {
        let mut c = Counter::<SuperGroup>::default();
        for _i in 0..=CoinsNb::MAX {
            let r = c.visit(&coin()).unwrap();
            assert_eq!(r, IterNext::Continue);
        }
        assert_eq!(c.last_index(), CoinsNb::MAX);
    }
}
