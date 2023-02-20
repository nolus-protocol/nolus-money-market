use serde::{Deserialize, Serialize};

use lpp::stub::lender::LppLenderRef;
use oracle::stub::OracleRef;

use crate::{
    api::{
        dex::ConnectionParams, opening::OngoingTrx, DownpaymentCoin, NewLeaseContract,
        StateResponse,
    },
    contract::{cmd::OpenLoanRespResult, state::ica_connector::IcaConnectee},
    dex::Account,
};

use super::transfer_out::TransferOut;

#[derive(Serialize, Deserialize)]
pub struct OpenIcaAccount {
    new_lease: NewLeaseContract,
    downpayment: DownpaymentCoin,
    loan: OpenLoanRespResult,
    deps: (LppLenderRef, OracleRef),
}

impl OpenIcaAccount {
    pub(super) fn new(
        new_lease: NewLeaseContract,
        downpayment: DownpaymentCoin,
        loan: OpenLoanRespResult,
        deps: (LppLenderRef, OracleRef),
    ) -> Self {
        Self {
            new_lease,
            downpayment,
            loan,
            deps,
        }
    }
}

impl IcaConnectee for OpenIcaAccount {
    type NextState = TransferOut;

    fn connected(self, dex_account: Account) -> Self::NextState {
        TransferOut::new(
            self.new_lease.form,
            dex_account,
            self.downpayment,
            self.loan,
            self.deps,
        )
    }

    fn dex(&self) -> &ConnectionParams {
        &self.new_lease.dex
    }
}

impl From<OpenIcaAccount> for StateResponse {
    fn from(value: OpenIcaAccount) -> Self {
        StateResponse::Opening {
            downpayment: value.downpayment,
            loan: value.loan.principal,
            loan_interest_rate: value.loan.annual_interest_rate,
            in_progress: OngoingTrx::OpenIcaAccount {},
        }
    }
}
