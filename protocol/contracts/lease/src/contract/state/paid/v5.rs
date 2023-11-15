use serde::Deserialize;

use currencies::LeaseGroup;
use currency::SymbolSlice;
use dex::{Account, CoinVisitor, IterNext, IterState, SwapTask};
use finance::coin::CoinDTO;
use oracle_platform::OracleRef;
use sdk::cosmwasm_std::{Env, QuerierWrapper};
use timealarms::stub::TimeAlarmsRef;

use crate::{
    api,
    contract::{
        finalize::FinalizerRef,
        state::{
            resp_delivery::{ForwardToDexEntry, ForwardToDexEntryContinue},
            SwapResult,
        },
        v5::Lease,
    },
    error::ContractResult,
    event::Type,
};

use super::{transfer_in::TransferIn as TransferIn_v6, Active as Active_v6};

pub(crate) type DexState =
    dex::StateLocalOut<TransferIn, ForwardToDexEntry, ForwardToDexEntryContinue>;

#[derive(Deserialize)]
pub(crate) struct Active {
    lease: Lease,
}

impl Active {
    pub(crate) fn migrate(self, finalizer: FinalizerRef) -> Active_v6 {
        Active_v6::new(self.lease.migrate(finalizer))
    }
}

#[derive(Deserialize)]
pub(crate) struct TransferIn {
    lease: Lease,
}

impl TransferIn {
    pub(crate) fn migrate(self, finalizer: FinalizerRef) -> TransferIn_v6 {
        TransferIn_v6::new(self.lease.migrate(finalizer))
    }
}

impl SwapTask for TransferIn {
    type OutG = LeaseGroup;
    type Label = Type;
    type StateResponse = ContractResult<api::StateResponse>;
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
        _querier: &QuerierWrapper<'_>,
    ) -> Self::Result {
        unreachable!()
    }
}
