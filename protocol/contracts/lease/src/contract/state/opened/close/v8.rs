use serde::Deserialize;

use currencies::Lpns;
use currency::SymbolSlice;
use dex::{Account, CoinVisitor, IterNext, IterState, SwapTask};
use finance::coin::CoinDTO;
use oracle::stub::OracleRef;
use sdk::cosmwasm_std::{Env, QuerierWrapper};
use timealarms::stub::TimeAlarmsRef;

use crate::{
    api::{query::StateResponse, LeasePaymentCurrencies},
    contract::{
        state::{
            opened::close::{customer_close, liquidation, SellAsset as SellAsset_v9},
            resp_delivery::{ForwardToDexEntry, ForwardToDexEntryContinue},
            SwapClient, SwapResult,
        },
        v8::Lease,
    },
    error::ContractResult,
    event::Type,
    finance::ReserveRef,
};

pub(crate) type PartialLiquidationDexState = DexState<liquidation::partial::RepayableImpl>;
pub(crate) type PartialLiquidationTask = SellAsset<liquidation::partial::RepayableImpl>;

pub(crate) type FullLiquidationDexState = DexState<liquidation::full::RepayableImpl>;
pub(crate) type FullLiquidationDexTask = SellAsset<liquidation::full::RepayableImpl>;

pub(crate) type PartialCloseDexState = DexState<customer_close::partial::RepayableImpl>;
pub(crate) type PartialCloseTask = SellAsset<customer_close::partial::RepayableImpl>;

pub(crate) type FullCloseDexState = DexState<customer_close::full::RepayableImpl>;
pub(crate) type FullCloseTask = SellAsset<customer_close::full::RepayableImpl>;

type DexState<Repayable> = dex::StateLocalOut<
    SellAsset<Repayable>,
    LeasePaymentCurrencies,
    SwapClient,
    ForwardToDexEntry,
    ForwardToDexEntryContinue,
>;

#[derive(Deserialize)]
pub(crate) struct SellAsset<RepayableImpl> {
    lease: Lease,
    repayable: RepayableImpl,
}

impl<RepayableImpl> SellAsset<RepayableImpl> {
    pub(crate) fn migrate(self, reserve: ReserveRef) -> SellAsset_v9<RepayableImpl> {
        SellAsset_v9::new(self.lease.migrate(reserve), self.repayable)
    }
}
impl<RepayableImpl> SwapTask for SellAsset<RepayableImpl> {
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
