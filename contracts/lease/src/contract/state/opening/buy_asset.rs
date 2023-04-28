use cosmwasm_std::{Env, QuerierWrapper, Timestamp};
use serde::{Deserialize, Serialize};

use currency::lease::LeaseGroup;
use dex::{
    Account, CoinVisitor, ContractInSwap, IterNext, IterState, StartLocalRemoteState, SwapState,
    SwapTask, TransferOutState,
};
use finance::{coin::CoinDTO, currency::Symbol};
use lpp::stub::lender::LppLenderRef;
use oracle::stub::OracleRef;
use platform::{
    ica::HostAccount, message::Response as MessageResponse,
    state_machine::Response as StateMachineResponse,
};
use timealarms::stub::TimeAlarmsRef;

use crate::{
    api::{self, opening::OngoingTrx, DownpaymentCoin, NewLeaseForm, StateResponse},
    contract::{
        cmd::{self, OpenLoanRespResult},
        state::{opened::active::Active, SwapResult},
        Lease,
    },
    error::ContractResult,
    event::Type,
    lease::IntoDTOResult,
};

type AssetGroup = LeaseGroup;
pub(super) type StartState = StartLocalRemoteState<BuyAsset>;
pub(in crate::contract::state) type DexState = dex::StateRemoteOut<BuyAsset>;

pub(in crate::contract::state::opening) fn start(
    form: NewLeaseForm,
    dex_account: Account,
    downpayment: DownpaymentCoin,
    loan: OpenLoanRespResult,
    deps: (LppLenderRef, OracleRef, TimeAlarmsRef),
) -> StartLocalRemoteState<BuyAsset> {
    dex::start_local_remote(BuyAsset::new(form, dex_account, downpayment, loan, deps))
}

type BuyAssetStateResponse = <BuyAsset as SwapTask>::StateResponse;

#[derive(Serialize, Deserialize)]
pub(crate) struct BuyAsset {
    form: NewLeaseForm,
    dex_account: Account,
    downpayment: DownpaymentCoin,
    loan: OpenLoanRespResult,
    deps: (LppLenderRef, OracleRef, TimeAlarmsRef),
}

impl BuyAsset {
    #[cfg(feature = "migration")]
    pub(super) fn migrate_to(
        form: NewLeaseForm,
        dex_account: Account,
        downpayment: DownpaymentCoin,
        loan: OpenLoanRespResult,
        deps: (LppLenderRef, OracleRef, TimeAlarmsRef),
    ) -> Self {
        Self::new(form, dex_account, downpayment, loan, deps)
    }

    fn new(
        form: NewLeaseForm,
        dex_account: Account,
        downpayment: DownpaymentCoin,
        loan: OpenLoanRespResult,
        deps: (LppLenderRef, OracleRef, TimeAlarmsRef),
    ) -> Self {
        Self {
            form,
            dex_account,
            downpayment,
            loan,
            deps,
        }
    }

    fn state<InP>(self, in_progress_fn: InP) -> BuyAssetStateResponse
    where
        InP: FnOnce(String) -> OngoingTrx,
    {
        Ok(StateResponse::Opening {
            downpayment: self.downpayment,
            loan: self.loan.principal,
            loan_interest_rate: self.loan.annual_interest_rate,
            in_progress: in_progress_fn(HostAccount::from(self.dex_account).into()),
        })
    }

    // fn emit_ok(&self) -> Emitter {
    //     Emitter::of_type(Type::OpeningTransferOut)
    // }
}

impl SwapTask for BuyAsset {
    type OutG = AssetGroup;
    type Label = Type;
    type StateResponse = ContractResult<api::StateResponse>;
    type Result = SwapResult;

    fn label(&self) -> Self::Label {
        Type::OpeningSwap
    }

    fn dex_account(&self) -> &Account {
        &self.dex_account
    }

    fn oracle(&self) -> &OracleRef {
        &self.deps.1
    }

    fn time_alarm(&self) -> &TimeAlarmsRef {
        &self.deps.2
    }

    fn out_currency(&self) -> Symbol<'_> {
        &self.form.currency
    }

    fn on_coins<Visitor>(&self, visitor: &mut Visitor) -> Result<IterState, Visitor::Error>
    where
        Visitor: CoinVisitor<Result = IterNext>,
    {
        dex::on_coins(&self.downpayment, &self.loan.principal, visitor)
    }

    fn finish(
        self,
        amount_out: CoinDTO<Self::OutG>,
        env: &Env,
        querier: &QuerierWrapper<'_>,
    ) -> Self::Result {
        let IntoDTOResult { lease, batch } = cmd::open_lease(
            self.form,
            self.dex_account.owner().clone(),
            env.block.time,
            &amount_out,
            querier,
            self.deps,
        )?;

        let active = Active::new(Lease {
            lease,
            dex: self.dex_account,
        });
        let emitter = active.emit_ok(env, self.downpayment, self.loan);
        Ok(StateMachineResponse::from(
            MessageResponse::messages_with_events(batch, emitter),
            active,
        ))
    }
}

impl ContractInSwap<TransferOutState, BuyAssetStateResponse> for BuyAsset {
    fn state(self, _now: Timestamp, _querier: &QuerierWrapper<'_>) -> BuyAssetStateResponse {
        let in_progress_fn = |ica_account| OngoingTrx::TransferOut { ica_account };
        self.state(in_progress_fn)
    }
}

impl ContractInSwap<SwapState, BuyAssetStateResponse> for BuyAsset {
    fn state(self, _now: Timestamp, _querier: &QuerierWrapper<'_>) -> BuyAssetStateResponse {
        let in_progress_fn = |ica_account| OngoingTrx::BuyAsset { ica_account };
        self.state(in_progress_fn)
    }
}
