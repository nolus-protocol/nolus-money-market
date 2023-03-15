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
        // TODO do not add a trx if the coin is of the same lease currency
        struct SwapWorker<'a>(SwapTrx<'a>, Symbol<'a>);
        impl<'a> CoinVisitor for SwapWorker<'a> {
            type Result = IterNext;
            type Error = ContractError;

            fn visit<G>(&mut self, coin: &CoinDTO<G>) -> Result<Self::Result, Self::Error>
            where
                G: Group,
            {
                self.0.swap_exact_in(coin, self.1)?;
                Ok(IterNext::Continue)
            }
        }

        let mut swapper = SwapWorker(swap_trx, self.spec.out_currency());
        let _res = self.spec.on_coins(&mut swapper)?;
        #[cfg(debug_assertions)]
        {
            debug_assert_eq!(_res, IterState::Complete);
        }
        Ok(swapper.0.into())
    }

    fn decode_response(&self, resp: &[u8], spec: &SwapTask) -> ContractResult<CoinDTO<OutG>> {
        struct ExactInResponse<I>(I, Amount);
        impl<I> CoinVisitor for ExactInResponse<I>
        where
            I: Iterator<Item = MsgData>,
        {
            type Result = IterNext;
            type Error = swap::error::Error;

            fn visit<G>(&mut self, _coin: &CoinDTO<G>) -> Result<Self::Result, Self::Error>
            where
                G: Group,
            {
                //TODO take into account the input amounts with currency == out_currency
                self.1 += swap_trx::exact_amount_in_resp(&mut self.0)?;
                Ok(IterNext::Continue)
            }
        }
        let mut resp = ExactInResponse(trx::decode_msg_responses(resp)?, Amount::ZERO);
        let _res = self.spec.on_coins(&mut resp)?;
        #[cfg(debug_assertions)]
        {
            debug_assert_eq!(_res, IterState::Complete);
        }

        coin::from_amount_ticker(resp.1, spec.out_currency()).map_err(Into::into)
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
    IcaConnector<InRecovery<Self>>: Into<State>,
{
    fn on_response(self, resp: Binary, deps: Deps<'_>, env: Env) -> ContractResult<Response> {
        // TODO transfer (downpayment - transferred_and_swapped), i.e. the nls_swap_fee to the profit
        let amount = self.decode_response(resp.as_slice(), &self.spec)?;
        self.spec.finish(amount, &deps.querier, env)
    }

    fn on_timeout(self, deps: Deps<'_>, env: Env) -> ContractResult<Response> {
        let state_label = self.spec.label();
        state::on_timeout_repair_channel(self, state_label, deps, env)
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
