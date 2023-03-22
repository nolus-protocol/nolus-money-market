use std::marker::PhantomData;

use cosmwasm_std::{Deps, Env, QuerierWrapper, Timestamp};
use serde::{Deserialize, Serialize};

use finance::{
    coin::{self, Amount, CoinDTO},
    currency::{Group, Symbol},
    zero::Zero,
};
use platform::{batch::Batch, trx};
use sdk::{cosmos_sdk_proto::cosmos::base::abci::v1beta1::MsgData, cosmwasm_std::Binary};
use swap::trx as swap_trx;

#[cfg(debug_assertions)]
use crate::contract::state::opening::swap_task::IterState;
use crate::{
    api::{dex::ConnectionParams, StateResponse},
    contract::{
        dex::{DexConnectable, SwapTrx},
        state::{
            self,
            controller::Controller,
            ica_connector::{Enterable, IcaConnector},
            ica_post_connector::Postpone,
            ica_recover::InRecovery,
            opening::swap_task::{CoinVisitor, IterNext, SwapTask as SwapTaskT},
            Response, State,
        },
        Contract,
    },
    error::{ContractError, ContractResult},
};

use super::{
    swap_state::{ContractInSwap, SwapState},
    swap_task::{OutChain, REMOTE_OUT_CHAIN},
};

#[derive(Serialize, Deserialize)]
pub(crate) struct SwapExactIn<OutG, SwapTask, const SWAP_OUT_CHAIN: OutChain> {
    spec: SwapTask,
    _out_g: PhantomData<OutG>,
}

impl<OutG, SwapTask, const SWAP_OUT_CHAIN: OutChain> SwapExactIn<OutG, SwapTask, SWAP_OUT_CHAIN> {
    pub(super) fn new(spec: SwapTask) -> Self {
        Self {
            spec,
            _out_g: PhantomData,
        }
    }
}

impl<OutG, SwapTask, const SWAP_OUT_CHAIN: OutChain> SwapExactIn<OutG, SwapTask, SWAP_OUT_CHAIN>
where
    OutG: Group,
    SwapTask: SwapTaskT<OutG>,
{
    pub(super) fn enter_state(
        &self,
        _now: Timestamp,
        querier: &QuerierWrapper<'_>,
    ) -> ContractResult<Batch> {
        let swap_trx = self.spec.dex_account().swap(self.spec.oracle(), querier);
        // TODO apply nls_swap_fee on the downpayment only!

        let mut builder = TrxBuilder(swap_trx, self.spec.out_currency(), false);
        let _res = self.spec.on_coins(&mut builder)?;
        #[cfg(debug_assertions)]
        {
            self.debug_check(builder.some(), _res);
        }

        Ok(builder.0.into())
    }

    fn decode_response(&self, resp: &[u8], spec: &SwapTask) -> ContractResult<CoinDTO<OutG>> {
        let mut parser =
            ResponseParser::new(trx::decode_msg_responses(resp)?, self.spec.out_currency());
        let _res = self.spec.on_coins(&mut parser)?;
        #[cfg(debug_assertions)]
        {
            self.debug_check(parser.some(), _res);
        }

        coin::from_amount_ticker(parser.total_amount(), spec.out_currency()).map_err(Into::into)
    }

    #[cfg(debug_assertions)]
    fn debug_check(&self, some: bool, res: IterState) {
        debug_assert!(
            some,
            "No coins with currency != {}",
            self.spec.out_currency()
        );
        debug_assert_eq!(res, IterState::Complete);
    }
}

impl<OutG, SwapTask, const SWAP_OUT_CHAIN: OutChain> Enterable
    for SwapExactIn<OutG, SwapTask, SWAP_OUT_CHAIN>
where
    OutG: Group,
    SwapTask: SwapTaskT<OutG>,
{
    fn enter(&self, deps: Deps<'_>, env: Env) -> ContractResult<Batch> {
        self.enter_state(env.block.time, &deps.querier)
    }
}

impl<OutG, SwapTask, const SWAP_OUT_CHAIN: OutChain> DexConnectable
    for SwapExactIn<OutG, SwapTask, SWAP_OUT_CHAIN>
where
    OutG: Group,
    SwapTask: SwapTaskT<OutG>,
{
    fn dex(&self) -> &ConnectionParams {
        self.spec.dex_account().dex()
    }
}

// impl<OutG, SwapTask> Controller for SwapExactIn<OutG, SwapTask, { LOCAL_OUT_CHAIN }>
// where
//     OutG: Group,
//     SwapTask: SwapTaskT<OutG>,
// {
//     fn on_response(self, _resp: Binary, _deps: Deps<'_>, _env: Env) -> ContractResult<Response> {
//         todo!()
//     }

//     fn on_timeout(self, deps: Deps<'_>, env: Env) -> ContractResult<Response> {
//         state::on_timeout_repair_channel(self, crate::event::Type::RepaymentTransferIn, deps, env)
//     }
// }

impl<OutG, SwapTask> Controller for SwapExactIn<OutG, SwapTask, { REMOTE_OUT_CHAIN }>
where
    OutG: Group,
    SwapTask: SwapTaskT<OutG, Result = Response, Error = ContractError>,
    Self: Into<State>,
    IcaConnector<false, InRecovery<Self>>: Into<State>,
{
    fn on_response(self, resp: Binary, deps: Deps<'_>, env: Env) -> ContractResult<Response> {
        // TODO transfer (downpayment - transferred_and_swapped), i.e. the nls_swap_fee to the profit
        let amount = self.decode_response(resp.as_slice(), &self.spec)?;
        self.spec.finish(amount, &deps.querier, env)
    }

    fn on_timeout(self, _deps: Deps<'_>, env: Env) -> ContractResult<Response> {
        let state_label = self.spec.label();
        state::on_timeout_repair_channel(self, state_label, env)
    }
}

impl<OutG, SwapTask, const SWAP_OUT_CHAIN: OutChain> Contract
    for SwapExactIn<OutG, SwapTask, SWAP_OUT_CHAIN>
where
    SwapTask: ContractInSwap<SwapState>,
{
    fn state(self, now: Timestamp, querier: &QuerierWrapper<'_>) -> ContractResult<StateResponse> {
        self.spec.state(now, querier)
    }
}

impl<OutG, SwapTask, const SWAP_OUT_CHAIN: OutChain> Postpone
    for SwapExactIn<OutG, SwapTask, SWAP_OUT_CHAIN>
where
    SwapTask: SwapTaskT<OutG>,
    ContractError: From<SwapTask::Error>,
{
    fn setup_alarm(&self, when: Timestamp, querier: &QuerierWrapper<'_>) -> ContractResult<Batch> {
        let time_alarms = self.spec.time_alarm(querier)?;
        time_alarms.setup_alarm(when).map_err(Into::into)
    }
}

struct TrxBuilder<'a>(SwapTrx<'a>, Symbol<'a>, bool);
impl<'a> TrxBuilder<'a> {
    #[cfg(debug_assertions)]
    fn some(&self) -> bool {
        self.2
    }
}
impl<'a> CoinVisitor for TrxBuilder<'a> {
    type Result = IterNext;
    type Error = ContractError;

    fn visit<G>(&mut self, coin: &CoinDTO<G>) -> Result<Self::Result, Self::Error>
    where
        G: Group,
    {
        if coin.ticker() != self.1 {
            self.0.swap_exact_in(coin, self.1)?;
            self.2 = true;
        }
        Ok(IterNext::Continue)
    }
}

struct ResponseParser<'a, I>(I, Symbol<'a>, Amount, bool);
impl<'a, I> ResponseParser<'a, I> {
    fn new(msgs: I, out: Symbol<'a>) -> Self {
        Self(msgs, out, Amount::ZERO, false)
    }
    fn total_amount(&self) -> Amount {
        self.2
    }
    #[cfg(debug_assertions)]
    fn some(&self) -> bool {
        self.3
    }
}
impl<'a, I> CoinVisitor for ResponseParser<'a, I>
where
    I: Iterator<Item = MsgData>,
{
    type Result = IterNext;
    type Error = swap::error::Error;

    fn visit<G>(&mut self, coin: &CoinDTO<G>) -> Result<Self::Result, Self::Error>
    where
        G: Group,
    {
        if coin.ticker() == self.1 {
            self.2 += coin.amount();
        } else {
            self.2 += swap_trx::exact_amount_in_resp(&mut self.0)?;
            self.3 = true;
        }
        Ok(IterNext::Continue)
    }
}
