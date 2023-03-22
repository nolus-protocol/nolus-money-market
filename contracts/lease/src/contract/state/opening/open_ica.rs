use cosmwasm_std::{QuerierWrapper, Timestamp};
use serde::{Deserialize, Serialize};

use lpp::stub::lender::LppLenderRef;
use oracle::stub::OracleRef;

use crate::{
    api::{
        dex::ConnectionParams, opening::OngoingTrx, DownpaymentCoin, NewLeaseContract,
        StateResponse,
    },
    contract::{
        cmd::OpenLoanRespResult,
        dex::{Account, DexConnectable},
        state::ica_connector::IcaConnectee,
        Contract,
    },
    error::ContractResult,
};

use super::buy_asset::{BuyAsset, Transfer};

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
    /// the next transaction is carried on the ICS20 channel so good to go
    const PRECONNECTABLE: bool = true;
    type NextState = Transfer;

    fn connected(self, dex_account: Account) -> Self::NextState {
        Self::NextState::new(BuyAsset::new(
            self.new_lease.form,
            dex_account,
            self.downpayment,
            self.loan,
            self.deps,
        ))
    }
}

impl DexConnectable for OpenIcaAccount {
    fn dex(&self) -> &ConnectionParams {
        &self.new_lease.dex
    }
}

impl Contract for OpenIcaAccount {
    fn state(
        self,
        _now: Timestamp,
        _querier: &QuerierWrapper<'_>,
    ) -> ContractResult<StateResponse> {
        Ok(StateResponse::Opening {
            downpayment: self.downpayment,
            loan: self.loan.principal,
            loan_interest_rate: self.loan.annual_interest_rate,
            in_progress: OngoingTrx::OpenIcaAccount {},
        })
    }
}
