use cosmwasm_std::Deps;
use serde::{Deserialize, Serialize};

use lpp::stub::lender::LppLenderRef;
use oracle::stub::OracleRef;
use platform::{bank_ibc::local::Sender, ica::HostAccount};

use crate::{
    api::{opening::OngoingTrx, DownpaymentCoin, NewLeaseForm, StateResponse},
    contract::{
        cmd::OpenLoanRespResult,
        state::{transfer_out::TransferOut as TransferOutT, BuyAsset, Response},
    },
    error::ContractResult,
};

#[derive(Serialize, Deserialize)]
pub struct TransferOut {
    form: NewLeaseForm,
    downpayment: DownpaymentCoin,
    loan: OpenLoanRespResult,
    dex_account: HostAccount,
    deps: (LppLenderRef, OracleRef),
}

impl TransferOut {
    //TODO change to super or crate::contract::state::opening once the other opening states have moved to opening module
    pub(in crate::contract::state) fn new(
        form: NewLeaseForm,
        downpayment: DownpaymentCoin,
        loan: OpenLoanRespResult,
        dex_account: HostAccount,
        deps: (LppLenderRef, OracleRef),
    ) -> Self {
        Self {
            form,
            downpayment,
            loan,
            dex_account,
            deps,
        }
    }
}

impl<'a> TransferOutT<'a> for TransferOut {
    fn channel<'b: 'a>(&'b self) -> &'a str {
        &self.form.dex.transfer_channel.local_endpoint
    }

    fn receiver(&self) -> HostAccount {
        self.dex_account.clone()
    }

    fn send(&self, sender: &mut Sender) -> ContractResult<()> {
        // TODO apply nls_swap_fee on the downpayment only!
        sender.send(&self.downpayment)?;
        sender.send(&self.loan.principal)?;
        Ok(())
    }

    fn on_success(self, platform: &Deps) -> ContractResult<Response> {
        let next_state = BuyAsset::new(
            self.form,
            self.downpayment,
            self.loan,
            self.dex_account,
            self.deps,
        );
        let batch = next_state.enter_state(&platform.querier)?;
        Ok(Response::from(batch, next_state))
    }

    fn into_state(self) -> StateResponse {
        StateResponse::Opening {
            downpayment: self.downpayment,
            loan: self.loan.principal,
            loan_interest_rate: self.loan.annual_interest_rate,
            in_progress: OngoingTrx::TransferOut {
                ica_account: self.dex_account.into(),
            },
        }
    }
}
