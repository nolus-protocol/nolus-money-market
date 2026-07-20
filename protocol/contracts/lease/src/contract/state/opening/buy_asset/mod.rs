use calculator::Factory as CalculatorFactory;
use currency::{CurrencyDef, Group, MemberOf};
use finish::BuyAssetFinish;
use oracle::stub::SwapPath;
use platform::remote::Account as RemoteAccount;
use serde::{Deserialize, Serialize};

use dex::{
    Account, ContractInSwap, Error as DexError, Stage, StartLocalRemoteState, SwapCoins,
    SwapOutputTask, SwapTask, WithCalculator, WithOutputTask,
};
use finance::duration::Duration;
use finance::instant::Instant;
use sdk::cosmwasm_std::{MessageInfo, QuerierWrapper};
use timealarms::stub::TimeAlarmsRef;

use crate::{
    api::{
        DownpaymentCoin, LeaseAssetCurrencies, LeasePaymentCurrencies,
        open::NewLeaseForm,
        query::{StateResponse as QueryStateResponse, opening::OngoingTrx},
    },
    contract::{
        cmd::OpenLoanRespResult,
        finalize::LeasesRef,
        state::{
            SwapResult,
            out_task::{OutTaskFactory, WithOutCurrency},
            resp_delivery::ForwardToDexEntry,
        },
        transport::{SwapClientFactory, TransferOutFactory},
    },
    error::ContractResult,
    event::Type,
    finance::{LppRef, OracleRef},
};

mod calculator;
mod finish;

type AssetGroup = LeaseAssetCurrencies;
#[allow(dead_code)]
pub(super) type StartState =
    StartLocalRemoteState<BuyAsset, TransferOutFactory, SwapClientFactory, ForwardToDexEntry>;
pub(in super::super) type DexState =
    dex::StateRemoteOut<BuyAsset, TransferOutFactory, SwapClientFactory, ForwardToDexEntry>;

pub(super) fn start(
    new_lease: NewLeaseForm,
    dex_account: Account,
    downpayment: DownpaymentCoin,
    loan: OpenLoanRespResult,
    deps: (LppRef, OracleRef, TimeAlarmsRef, LeasesRef),
    start_opening_at: Instant,
) -> StartState {
    dex::start_local_remote(
        BuyAsset::new(
            new_lease,
            dex_account,
            downpayment,
            loan,
            deps,
            start_opening_at,
        ),
        TransferOutFactory::default(),
        SwapClientFactory::default(),
    )
}

type BuyAssetStateResponse = <BuyAsset as SwapTask>::StateResponse;

#[derive(Serialize, Deserialize)]
pub struct BuyAsset {
    form: NewLeaseForm,
    dex_account: Account,
    downpayment: DownpaymentCoin,
    loan: OpenLoanRespResult,
    deps: (LppRef, OracleRef, TimeAlarmsRef, LeasesRef),
    start_opening_at: Instant,
}

impl BuyAsset {
    pub(super) fn new(
        form: NewLeaseForm,
        dex_account: Account,
        downpayment: DownpaymentCoin,
        loan: OpenLoanRespResult,
        deps: (LppRef, OracleRef, TimeAlarmsRef, LeasesRef),
        start_opening_at: Instant,
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

    fn state<InP>(self, in_progress: InP) -> BuyAssetStateResponse
    where
        InP: FnOnce(RemoteAccount) -> OngoingTrx,
    {
        Ok(QueryStateResponse::Opening {
            currency: self.form.currency,
            downpayment: self.downpayment,
            loan: self.loan.principal,
            loan_interest_rate: self.loan.annual_interest_rate,
            in_progress: in_progress(self.dex_account.into()),
        })
    }
}

impl SwapTask for BuyAsset {
    type InG = LeasePaymentCurrencies;
    type OutG = AssetGroup;
    type Label = Type;
    type StateResponse = ContractResult<QueryStateResponse>;
    type Result = SwapResult;

    fn label(&self) -> Self::Label {
        Type::OpeningSwap
    }

    fn dex_account(&self) -> &Account {
        &self.dex_account
    }

    fn oracle(&self) -> &impl SwapPath<<Self::InG as Group>::TopG> {
        &self.deps.1
    }

    fn time_alarm(&self) -> &TimeAlarmsRef {
        &self.deps.2
    }

    fn authz_remote_callback(
        &self,
        querier: QuerierWrapper<'_>,
        info: &MessageInfo,
    ) -> dex::DexResult<()> {
        access_control::check(&self.deps.3.remote_lease_callback_permission(querier), info)
            .map_err(DexError::Unauthorized)
    }

    fn coins(&self) -> SwapCoins<Self::InG> {
        SwapCoins::Two(self.downpayment, self.loan.principal.into_super_group())
    }

    fn with_slippage_calc<WithCalc>(&self, with_calc: WithCalc) -> WithCalc::Output
    where
        WithCalc: WithCalculator<Self>,
    {
        self.form
            .currency
            .into_super_group()
            .into_currency_type(CalculatorFactory::from(with_calc))
    }

    fn into_output_task<Cmd>(self, cmd: Cmd) -> Cmd::Output
    where
        Cmd: WithOutputTask<Self>,
    {
        struct OutputTaskFactory {}
        impl OutTaskFactory<BuyAsset> for OutputTaskFactory {
            fn new_task<OutC>(swap_task: BuyAsset) -> impl SwapOutputTask<BuyAsset, OutC = OutC>
            where
                OutC: CurrencyDef,
                OutC::Group: MemberOf<<BuyAsset as SwapTask>::OutG>
                    + MemberOf<<<BuyAsset as SwapTask>::InG as Group>::TopG>,
            {
                BuyAssetFinish::<_, OutC>::from(swap_task)
            }
        }
        self.form
            .currency
            .into_super_group()
            .into_currency_type(WithOutCurrency::<_, OutputTaskFactory, _>::from(self, cmd))
    }
}

impl ContractInSwap for BuyAsset {
    type StateResponse = <Self as SwapTask>::StateResponse;

    fn state(
        self,
        in_progress: Stage,
        _now: Instant,
        _due_projection: Duration,
        _querier: QuerierWrapper<'_>,
    ) -> Self::StateResponse {
        match in_progress {
            Stage::TransferOut => {
                self.state(|remote_lease| OngoingTrx::TransferOut { remote_lease })
            }
            Stage::Swap => self.state(|remote_lease| OngoingTrx::BuyAsset { remote_lease }),
            Stage::TransferInInit => unimplemented!(),
            Stage::TransferInFinish => unimplemented!(),
        }
    }
}
