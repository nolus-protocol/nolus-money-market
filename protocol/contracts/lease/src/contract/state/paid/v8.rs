use serde::Deserialize;

use currencies::LeaseGroup;
use currency::SymbolSlice;
use dex::{Account, CoinVisitor, IterNext, IterState, SwapTask};
use finance::coin::CoinDTO;
use sdk::cosmwasm_std::{Env, QuerierWrapper};
use timealarms::stub::TimeAlarmsRef;

use crate::{
    api::{query::StateResponse, LeasePaymentCurrencies},
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

use super::{transfer_in::TransferIn as TransferIn_v9, Active as Active_v9};

pub(crate) type DexState = dex::StateLocalOut<
    TransferIn,
    LeasePaymentCurrencies,
    SwapClient,
    ForwardToDexEntry,
    ForwardToDexEntryContinue,
>;

#[derive(Deserialize)]
pub(crate) struct Active {
    lease: Lease,
}

impl Active {
    pub(crate) fn migrate(self, reserve: ReserveRef) -> Active_v9 {
        Active_v9::new(self.lease.migrate(reserve))
    }
}

#[derive(Deserialize)]
pub(crate) struct TransferIn {
    lease: Lease,
}

impl TransferIn {
    pub(crate) fn migrate(self, reserve: ReserveRef) -> TransferIn_v9 {
        TransferIn_v9::new(self.lease.migrate(reserve))
    }
}

impl SwapTask for TransferIn {
    type OutG = LeaseGroup;
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
