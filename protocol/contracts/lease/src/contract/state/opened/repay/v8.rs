use serde::Deserialize;

use currencies::Lpns;
use currency::SymbolSlice;
use dex::{Account, CoinVisitor, IterNext, IterState, SwapTask};
use finance::coin::CoinDTO;
use sdk::cosmwasm_std::{Env, QuerierWrapper};
use timealarms::stub::TimeAlarmsRef;

use crate::{
    api::{query::StateResponse, LeasePaymentCurrencies, PaymentCoin},
    contract::{
        state::{
            resp_delivery::{ForwardToDexEntry, ForwardToDexEntryContinue},
            SwapClient, SwapResult,
        },
        v8::Lease,
    },
    error::ContractResult,
    event::Type,
    finance::{OracleRef, ReserveRef},
};

use super::buy_lpn::BuyLpn as BuyLpn_v9;

pub(crate) type DexState = dex::StateLocalOut<
    BuyLpn,
    LeasePaymentCurrencies,
    SwapClient,
    ForwardToDexEntry,
    ForwardToDexEntryContinue,
>;

#[derive(Deserialize)]
pub(crate) struct BuyLpn {
    lease: Lease,
    payment: PaymentCoin,
}

impl BuyLpn {
    pub(crate) fn migrate(self, reserve: ReserveRef) -> BuyLpn_v9 {
        BuyLpn_v9::migrate_to(self.lease.migrate(reserve), self.payment)
    }
}

impl SwapTask for BuyLpn {
    type OutG = Lpns;
    type Label = Type;
    type StateResponse = ContractResult<StateResponse>;
    type Result = SwapResult;

    fn label(&self) -> Self::Label {
        unreachable!()
    }

    fn dex_account(&self) -> &Account {
        unreachable!()
    }

    fn oracle(&self) -> &OracleRef {
        unreachable!()
    }

    fn time_alarm(&self) -> &TimeAlarmsRef {
        unreachable!()
    }

    fn out_currency(&self) -> &SymbolSlice {
        unreachable!()
    }

    fn on_coins<Visitor>(&self, _visitor: &mut Visitor) -> Result<IterState, Visitor::Error>
    where
        Visitor: CoinVisitor<Result = IterNext>,
    {
        unreachable!()
    }

    fn finish(
        self,
        _amount_out: CoinDTO<Self::OutG>,
        _env: &Env,
        _querier: QuerierWrapper<'_>,
    ) -> Self::Result {
        unreachable!()
    }
}
