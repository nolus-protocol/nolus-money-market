use cosmwasm_std::{Env, QuerierWrapper, Timestamp};
use currency::lpn::Lpns;
use dex::{
    Account, CoinVisitor, ContractInSwap, IterNext, IterState, StartRemoteLocalState, SwapState,
    SwapTask, TransferInFinishState, TransferInInitState, TransferOutState,
};
use finance::{coin::CoinDTO, currency::Symbol};
use oracle::stub::OracleRef;
use serde::{Deserialize, Serialize};
use timealarms::stub::TimeAlarmsRef;

use crate::{
    api::{self, opened::LiquidateTrx},
    contract::{
        state::{opened::active::Active, SwapResult},
        Lease,
    },
    error::ContractResult,
    event::Type,
    lease::LiquidationDTO,
};

pub(crate) type DexState = dex::StateLocalOut<SellAsset>;

pub(in crate::contract::state) fn start(
    lease: Lease,
    liquidation: LiquidationDTO,
) -> StartRemoteLocalState<SellAsset> {
    dex::start_remote_local(SellAsset::new(lease, liquidation))
}

type SellAssetStateResponse = <SellAsset as SwapTask>::StateResponse;

#[derive(Serialize, Deserialize)]
pub(crate) struct SellAsset {
    lease: Lease,
    liquidation: LiquidationDTO,
}

impl SellAsset {
    pub(in crate::contract::state) fn new(lease: Lease, liquidation: LiquidationDTO) -> Self {
        Self { lease, liquidation }
    }
}

impl SwapTask for SellAsset {
    type OutG = Lpns;
    type Label = Type;
    type StateResponse = ContractResult<api::StateResponse>;
    type Result = SwapResult;

    fn label(&self) -> Self::Label {
        Type::LiquidationSwap
    }

    fn dex_account(&self) -> &Account {
        &self.lease.dex
    }

    fn oracle(&self) -> &OracleRef {
        &self.lease.lease.oracle
    }

    fn time_alarm(&self) -> &TimeAlarmsRef {
        &self.lease.lease.time_alarms
    }

    fn out_currency(&self) -> Symbol<'_> {
        self.lease.lease.loan.lpp().currency()
    }

    fn on_coins<Visitor>(&self, visitor: &mut Visitor) -> Result<IterState, Visitor::Error>
    where
        Visitor: CoinVisitor<Result = IterNext>,
    {
        dex::on_coin(self.liquidation.amount(&self.lease.lease), visitor)
    }

    fn finish(
        self,
        amount_out: CoinDTO<Self::OutG>,
        env: &Env,
        querier: &QuerierWrapper<'_>,
    ) -> Self::Result {
        Active::try_repay_lpn(self.lease, amount_out, querier, env)
    }
}

impl ContractInSwap<TransferOutState, SellAssetStateResponse> for SellAsset {
    fn state(self, _now: Timestamp, _querier: &QuerierWrapper<'_>) -> SellAssetStateResponse {
        // it's due to reusing the same enum dex::State
        // have to define a tailored enum dex::State that starts from SwapExactIn
        unreachable!(
            "The sell lease asset on liquidation task never goes through a 'TransferOut' state!"
        )
    }
}

impl ContractInSwap<SwapState, SellAssetStateResponse> for SellAsset {
    fn state(self, now: Timestamp, querier: &QuerierWrapper<'_>) -> SellAssetStateResponse {
        super::query(
            self.lease.lease,
            self.liquidation,
            LiquidateTrx::Swap,
            now,
            querier,
        )
    }
}

impl ContractInSwap<TransferInInitState, SellAssetStateResponse> for SellAsset {
    fn state(self, now: Timestamp, querier: &QuerierWrapper<'_>) -> SellAssetStateResponse {
        super::query(
            self.lease.lease,
            self.liquidation,
            LiquidateTrx::TransferInInit,
            now,
            querier,
        )
    }
}

impl ContractInSwap<TransferInFinishState, SellAssetStateResponse> for SellAsset {
    fn state(self, now: Timestamp, querier: &QuerierWrapper<'_>) -> SellAssetStateResponse {
        super::query(
            self.lease.lease,
            self.liquidation,
            LiquidateTrx::TransferInFinish,
            now,
            querier,
        )
    }
}
