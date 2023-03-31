use std::marker::PhantomData;

use cosmwasm_std::{Binary, Deps, Env, QuerierWrapper, Timestamp};
use platform::batch::{Batch, Emitter};
use serde::{Deserialize, Serialize};

use finance::{coin::CoinDTO, currency::Group, zero::Zero};

use crate::{
    api::StateResponse,
    contract::{
        dex::TransferOutTrx,
        state::{
            self,
            controller::Controller,
            ica_connector::Enterable,
            opening::{
                never::{self, Never},
                swap_task::{CoinVisitor, IterNext},
            },
            Response, State,
        },
        Contract,
    },
    error::{ContractError, ContractResult},
};

use super::{
    coin_index,
    swap_exact_in::SwapExactIn,
    swap_state::{ContractInSwap, TransferOutState},
    swap_task::{CoinsNb, IterState, OutChain, SwapTask as SwapTaskT},
};

/// Transfer out a coin to DEX
///
/// Supports up to `CoinsNb::MAX` number of coins.
#[derive(Serialize, Deserialize)]
pub(crate) struct TransferOut<OutG, SwapTask, const SWAP_OUT_CHAIN: OutChain> {
    spec: SwapTask,
    coin_index: CoinsNb,
    last_coin_index: CoinsNb,
    _out_g: PhantomData<OutG>,
}

impl<OutG, SwapTask, const SWAP_OUT_CHAIN: OutChain> TransferOut<OutG, SwapTask, SWAP_OUT_CHAIN>
where
    OutG: Group,
    SwapTask: SwapTaskT<OutG>,
{
    pub(super) fn new(spec: SwapTask) -> Self {
        let first_index = Default::default();
        let last_coin_index = Self::last_coin_index(&spec);
        Self::new_with_index(spec, first_index, last_coin_index)
    }

    fn next(self) -> Self {
        debug_assert!(!self.last_coin());

        debug_assert!(self.coin_index < CoinsNb::MAX); // though already checked implicitly with the `self.last_coin()`
        let next_index = self.coin_index + 1;

        Self::new_with_index(self.spec, next_index, self.last_coin_index)
    }

    fn new_with_index(spec: SwapTask, coin_index: CoinsNb, last_coin_index: CoinsNb) -> Self {
        debug_assert!(coin_index <= last_coin_index);
        Self {
            spec,
            coin_index,
            last_coin_index,
            _out_g: PhantomData,
        }
    }

    fn last_coin(&self) -> bool {
        debug_assert!(self.coin_index <= self.last_coin_index);
        self.coin_index == self.last_coin_index
    }

    fn last_coin_index(spec: &SwapTask) -> CoinsNb {
        let mut counter = Counter::default();
        let _res = never::safe_unwrap(spec.on_coins(&mut counter));

        #[cfg(debug_assert)]
        debug_assert_eq!(_res, IterState::Complete);
        counter.last_index()
    }

    fn enter_state(&self, now: Timestamp, _querier: &QuerierWrapper<'_>) -> ContractResult<Batch> {
        struct SendWorker<'a>(TransferOutTrx<'a>, bool);
        impl<'a> CoinVisitor for SendWorker<'a> {
            type Result = ();
            type Error = ContractError;

            fn visit<G>(&mut self, coin: &CoinDTO<G>) -> Result<Self::Result, Self::Error>
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
}

impl<OutG, SwapTask, const SWAP_OUT_CHAIN: OutChain> Enterable
    for TransferOut<OutG, SwapTask, SWAP_OUT_CHAIN>
where
    OutG: Group,
    SwapTask: SwapTaskT<OutG>,
{
    fn enter(&self, deps: Deps<'_>, env: &Env) -> ContractResult<Batch> {
        self.enter_state(env.block.time, &deps.querier)
    }
}

impl<OutG, SwapTask, const SWAP_OUT_CHAIN: OutChain> Controller
    for TransferOut<OutG, SwapTask, SWAP_OUT_CHAIN>
where
    OutG: Group,
    SwapTask: SwapTaskT<OutG>,
    Self: Into<State>,
    SwapExactIn<OutG, SwapTask, SWAP_OUT_CHAIN>: Into<State>,
{
    fn on_response(self, _resp: Binary, deps: Deps<'_>, env: Env) -> ContractResult<Response> {
        let emitter = Emitter::of_type(self.spec.label());
        if self.last_coin() {
            let swap = SwapExactIn::new(self.spec);
            let batch = swap.enter_state(env.block.time, &deps.querier)?;
            let resp = batch.into_response(emitter);

            Ok(Response::from(resp, swap))
        } else {
            let next_transfer = self.next();
            let batch = next_transfer.enter_state(env.block.time, &deps.querier)?;

            Ok(Response::from(batch.into_response(emitter), next_transfer))
        }
    }

    fn on_timeout(self, deps: Deps<'_>, env: Env) -> ContractResult<Response> {
        let state_label = self.spec.label();
        state::on_timeout_retry(self, state_label, deps, env)
    }
}

impl<OutG, SwapTask, const SWAP_OUT_CHAIN: OutChain> Contract
    for TransferOut<OutG, SwapTask, SWAP_OUT_CHAIN>
where
    SwapTask: ContractInSwap<TransferOutState>,
{
    fn state(self, now: Timestamp, querier: &QuerierWrapper<'_>) -> ContractResult<StateResponse> {
        self.spec.state(now, querier)
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

    fn visit<G>(&mut self, _coin: &CoinDTO<G>) -> Result<Self::Result, Self::Error>
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
    use finance::{
        coin::{Coin, CoinDTO},
        test::currency::{TestCurrencies, Usdc},
    };

    use crate::contract::state::opening::swap_task::{CoinVisitor, CoinsNb, IterNext};

    use super::Counter;

    fn coin() -> CoinDTO<TestCurrencies> {
        Coin::<Usdc>::new(22).into()
    }

    #[test]
    fn index_zero() {
        let mut c = Counter::default();
        let r = c.visit::<TestCurrencies>(&coin()).unwrap();
        assert_eq!(r, IterNext::Continue);
        assert_eq!(c.last_index(), 0);
    }

    #[test]
    fn index_one() {
        let mut c = Counter::default();
        let r = c.visit::<TestCurrencies>(&coin()).unwrap();
        assert_eq!(r, IterNext::Continue);
        let r = c.visit::<TestCurrencies>(&coin()).unwrap();
        assert_eq!(r, IterNext::Continue);
        assert_eq!(c.last_index(), 1);
    }

    #[test]
    fn index_max() {
        let mut c = Counter::default();
        for _i in 0..=CoinsNb::MAX {
            let r = c.visit::<TestCurrencies>(&coin()).unwrap();
            assert_eq!(r, IterNext::Continue);
        }
        assert_eq!(c.last_index(), CoinsNb::MAX);
    }
}
