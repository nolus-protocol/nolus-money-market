use oracle::stub::SwapPath;
use serde::{Deserialize, Serialize};

use currency::CurrencyDTO;
use dex::{
    Account, CoinVisitor, ContractInSwap, IterNext, IterState, StartLocalRemoteState, SwapState,
    SwapTask, TransferOutState,
};
use finance::coin::CoinDTO;
use platform::{
    ica::HostAccount, message::Response as MessageResponse,
    state_machine::Response as StateMachineResponse,
};
use sdk::cosmwasm_std::{Env, QuerierWrapper, Timestamp};
use timealarms::stub::TimeAlarmsRef;

use crate::{
    api::{
        open::{NewLeaseContract, NewLeaseForm},
        query::{opening::OngoingTrx, StateResponse as QueryStateResponse},
        DownpaymentCoin, LeaseAssetCurrencies, LeasePaymentCurrencies,
    },
    contract::{
        cmd::{self, OpenLoanRespResult},
        finalize::FinalizerRef,
        state::{
            opened::active::Active,
            resp_delivery::{ForwardToDexEntry, ForwardToDexEntryContinue},
            SwapClient, SwapResult,
        },
        Lease,
    },
    error::ContractResult,
    event::Type,
    finance::{LppRef, OracleRef},
    lease::IntoDTOResult,
};

use super::open_ica::OpenIcaAccount;

type AssetGroup = LeaseAssetCurrencies;
pub(super) type StartState = StartLocalRemoteState<OpenIcaAccount, BuyAsset>;
pub(in super::super) type DexState = dex::StateRemoteOut<
    OpenIcaAccount,
    BuyAsset,
    LeasePaymentCurrencies,
    SwapClient,
    ForwardToDexEntry,
    ForwardToDexEntryContinue,
>;

pub(in super::super::opening) fn start(
    new_lease: NewLeaseContract,
    downpayment: DownpaymentCoin,
    loan: OpenLoanRespResult,
    deps: (LppRef, OracleRef, TimeAlarmsRef, FinalizerRef),
    start_opening_at: Timestamp,
) -> StartState {
    dex::start_local_remote::<_, BuyAsset>(OpenIcaAccount::new(
        new_lease,
        downpayment,
        loan,
        deps,
        start_opening_at,
    ))
}

type BuyAssetStateResponse = <BuyAsset as SwapTask>::StateResponse;

#[derive(Serialize, Deserialize)]
pub(crate) struct BuyAsset {
    form: NewLeaseForm,
    dex_account: Account,
    downpayment: DownpaymentCoin,
    loan: OpenLoanRespResult,
    deps: (LppRef, OracleRef, TimeAlarmsRef, FinalizerRef),
    start_opening_at: Timestamp,
}

impl BuyAsset {
    pub(super) fn new(
        form: NewLeaseForm,
        dex_account: Account,
        downpayment: DownpaymentCoin,
        loan: OpenLoanRespResult,
        deps: (LppRef, OracleRef, TimeAlarmsRef, FinalizerRef),
        start_opening_at: Timestamp,
    ) -> Self {
        Self {
            form,
            dex_account,
            downpayment,
            loan,
            deps,
            start_opening_at,
        }
    }

    fn state<InP>(self, in_progress_fn: InP) -> BuyAssetStateResponse
    where
        InP: FnOnce(String) -> OngoingTrx,
    {
        Ok(QueryStateResponse::Opening {
            downpayment: self.downpayment,
            loan: self.loan.principal,
            loan_interest_rate: self.loan.annual_interest_rate,
            in_progress: in_progress_fn(HostAccount::from(self.dex_account).into()),
        })
    }
}

impl SwapTask for BuyAsset {
    type InG = LeasePaymentCurrencies;
    type OutG = AssetGroup;
    type InOutG = LeasePaymentCurrencies;
    type Label = Type;
    type StateResponse = ContractResult<QueryStateResponse>;
    type Result = SwapResult;

    fn label(&self) -> Self::Label {
        Type::OpeningSwap
    }

    fn dex_account(&self) -> &Account {
        &self.dex_account
    }

    fn oracle(&self) -> &impl SwapPath<Self::InOutG> {
        &self.deps.1
    }

    fn time_alarm(&self) -> &TimeAlarmsRef {
        &self.deps.2
    }

    fn out_currency(&self) -> CurrencyDTO<Self::OutG> {
        self.form.currency
    }

    fn on_coins<Visitor>(&self, visitor: &mut Visitor) -> Result<IterState, Visitor::Error>
    where
        Visitor: CoinVisitor<GIn = Self::InG, Result = IterNext>,
    {
        dex::on_coins(&self.downpayment, &self.loan.principal, visitor)
    }

    fn finish(
        self,
        amount_out: CoinDTO<Self::OutG>,
        env: &Env,
        querier: QuerierWrapper<'_>,
    ) -> Self::Result {
        let IntoDTOResult { lease, batch } = cmd::open_lease(
            self.form,
            self.dex_account.owner().clone(),
            self.start_opening_at,
            &env.block.time,
            amount_out,
            querier,
            (self.deps.0, self.deps.1, self.deps.2),
        )?;

        let active = Active::new(Lease::new(lease, self.dex_account, self.deps.3));
        let emitter = active.emit_opened(env, self.downpayment, self.loan);
        Ok(StateMachineResponse::from(
            MessageResponse::messages_with_events(batch, emitter),
            active,
        ))
    }
}

impl ContractInSwap<TransferOutState> for BuyAsset {
    type StateResponse = <Self as SwapTask>::StateResponse;

    fn state(self, _now: Timestamp, _querier: QuerierWrapper<'_>) -> Self::StateResponse {
        let in_progress_fn = |ica_account| OngoingTrx::TransferOut { ica_account };
        self.state(in_progress_fn)
    }
}

impl ContractInSwap<SwapState> for BuyAsset {
    type StateResponse = <Self as SwapTask>::StateResponse;

    fn state(self, _now: Timestamp, _querier: QuerierWrapper<'_>) -> Self::StateResponse {
        let in_progress_fn = |ica_account| OngoingTrx::BuyAsset { ica_account };
        self.state(in_progress_fn)
    }
}
