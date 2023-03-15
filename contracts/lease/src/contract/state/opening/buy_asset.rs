use cosmwasm_std::{Env, QuerierWrapper, Timestamp};
use currency::lease::LeaseGroup;
use finance::{coin::CoinDTO, currency::Symbol};
use lpp::stub::lender::LppLenderRef;
use oracle::stub::OracleRef;
use platform::ica::HostAccount;
use serde::{Deserialize, Serialize};

use crate::{
    api::{opening::OngoingTrx, DownpaymentCoin, NewLeaseForm, StateResponse},
    contract::{
        cmd::OpenLoanRespResult,
        dex::Account,
        state::{opened::active::Active, Response},
        Lease,
    },
    error::ContractError,
    event::Type,
    lease::IntoDTOResult,
};

use super::{
    swap_coins,
    swap_exact_in::SwapExactIn,
    swap_state::{ContractInSwap, SwapState, TransferOutState},
    swap_task::{
        CoinVisitor, IterNext, IterState, OutChain, SwapTask as SwapTaskT, REMOTE_OUT_CHAIN,
    },
    transfer_out::TransferOut,
};

const OUT_CHAIN: OutChain = REMOTE_OUT_CHAIN;
type AssetGroup = LeaseGroup;
pub(crate) type Transfer = TransferOut<AssetGroup, BuyAsset, OUT_CHAIN>;
pub(crate) type Swap = SwapExactIn<AssetGroup, BuyAsset, OUT_CHAIN>;

#[derive(Serialize, Deserialize)]
pub(crate) struct BuyAsset {
    form: NewLeaseForm,
    dex_account: Account,
    downpayment: DownpaymentCoin,
    loan: OpenLoanRespResult,
    deps: (LppLenderRef, OracleRef),
}

impl BuyAsset {
    pub(super) fn new(
        form: NewLeaseForm,
        dex_account: Account,
        downpayment: DownpaymentCoin,
        loan: OpenLoanRespResult,
        deps: (LppLenderRef, OracleRef),
    ) -> Self {
        Self {
            form,
            dex_account,
            downpayment,
            loan,
            deps,
        }
    }

    fn state<InP>(self, in_progress_fn: InP) -> StateResponse
    where
        InP: FnOnce(String) -> OngoingTrx,
    {
        StateResponse::Opening {
            downpayment: self.downpayment,
            loan: self.loan.principal,
            loan_interest_rate: self.loan.annual_interest_rate,
            in_progress: in_progress_fn(HostAccount::from(self.dex_account).into()),
        }
    }

    // fn emit_ok(&self) -> Emitter {
    //     Emitter::of_type(Type::OpeningTransferOut)
    // }
}

impl SwapTaskT<AssetGroup> for BuyAsset {
    type Result = Response;
    type Error = ContractError;
    type Label = Type;

    fn label(&self) -> Self::Label {
        Type::OpeningSwap
    }

    fn dex_account(&self) -> &Account {
        &self.dex_account
    }

    fn oracle(&self) -> &OracleRef {
        &self.deps.1
    }

    fn out_currency(&self) -> Symbol<'_> {
        &self.form.currency
    }

    fn on_coins<Visitor>(&self, visitor: &mut Visitor) -> Result<IterState, Visitor::Error>
    where
        Visitor: CoinVisitor<Result = IterNext>,
    {
        swap_coins::on_coins(&self.downpayment, &self.loan.principal, visitor)
    }

    fn finish(
        self,
        amount: CoinDTO<AssetGroup>,
        querier: &QuerierWrapper<'_>,
        env: Env,
    ) -> Result<Self::Result, Self::Error> {
        let IntoDTOResult { lease, batch } = self.form.into_lease(
            env.contract.address.clone(),
            env.block.time,
            &amount,
            querier,
            self.deps,
        )?;
        let active = Active::new(Lease {
            lease,
            dex: self.dex_account,
        });
        let emitter = active.emit_ok(&env, self.downpayment, self.loan);
        Ok(Response::from(batch.into_response(emitter), active))
    }
}

impl ContractInSwap<TransferOutState> for BuyAsset {
    fn state(
        self,
        _now: Timestamp,
        _querier: &QuerierWrapper<'_>,
    ) -> Result<StateResponse, ContractError> {
        let in_progress_fn = |ica_account| OngoingTrx::TransferOut { ica_account };
        Ok(self.state(in_progress_fn))
    }
}

impl ContractInSwap<SwapState> for BuyAsset {
    fn state(
        self,
        _now: Timestamp,
        _querier: &QuerierWrapper<'_>,
    ) -> Result<StateResponse, ContractError> {
        let in_progress_fn = |ica_account| OngoingTrx::BuyAsset { ica_account };
        Ok(self.state(in_progress_fn))
    }
}
