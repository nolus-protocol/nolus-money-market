use serde::Deserialize;

use currency::{lpn::Lpns, SymbolSlice};
use dex::{Account, CoinVisitor, IterNext, IterState, SwapTask};
use finance::coin::CoinDTO;
use oracle::stub::OracleRef;
use sdk::cosmwasm_std::{Env, QuerierWrapper};
use timealarms::stub::TimeAlarmsRef;

use crate::{
    api::{self, LeaseCoin},
    contract::{
        cmd::{
            FullLiquidationDTO as FullLiquidationDTO_v6,
            PartialLiquidationDTO as PartialLiquidationDTO_v6,
        },
        finalize::FinalizerRef,
        state::{
            opened::close::SellAsset as SellAsset_v6,
            resp_delivery::{ForwardToDexEntry, ForwardToDexEntryContinue},
            SwapResult,
        },
        v5::Lease,
    },
    error::ContractResult,
    event::Type,
    position::Cause,
};

use super::{full, partial};

pub(crate) type DexState =
    dex::StateLocalOut<SellAsset, ForwardToDexEntry, ForwardToDexEntryContinue>;

#[derive(Deserialize)]
pub(crate) struct SellAsset {
    lease: Lease,
    liquidation: LiquidationDTO,
}

#[derive(Deserialize)]
pub(crate) enum LiquidationDTO {
    Partial { amount: LeaseCoin, cause: Cause },
    Full(Cause),
}

impl SellAsset {
    pub(crate) fn partial(&self) -> bool {
        matches!(
            self.liquidation,
            LiquidationDTO::Partial {
                amount: _,
                cause: _
            }
        )
    }

    pub(crate) fn migrate_into_partial(
        self,
        finalizer: FinalizerRef,
    ) -> SellAsset_v6<partial::RepayableImpl> {
        if let LiquidationDTO::Partial { amount, cause } = self.liquidation {
            SellAsset_v6::new(
                self.lease.migrate(finalizer),
                Into::into(PartialLiquidationDTO_v6 { amount, cause }),
            )
        } else {
            unreachable!("This SellAsset should have been migrated as a full liquidation!")
        }
    }

    pub(crate) fn migrate_into_full(
        self,
        finalizer: FinalizerRef,
    ) -> SellAsset_v6<full::RepayableImpl> {
        if let LiquidationDTO::Full(cause) = self.liquidation {
            SellAsset_v6::new(
                self.lease.migrate(finalizer),
                Into::into(FullLiquidationDTO_v6 { cause }),
            )
        } else {
            unreachable!("This SellAsset should have been migrated as a partial liquidation!")
        }
    }
}
impl SwapTask for SellAsset {
    type OutG = Lpns;
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
