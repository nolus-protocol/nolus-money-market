use serde::Serialize;

use finance::currency::Currency;
use lpp::stub::loan::LppLoan as LppLoanTrait;
use oracle::stub::Oracle as OracleTrait;
use profit::stub::Profit as ProfitTrait;
use sdk::cosmwasm_std::Timestamp;

use crate::{
    api::{opened::OngoingTrx, StateResponse},
    error::ContractError,
    lease::{with_lease::WithLease, Lease},
};

pub struct LeaseState {
    now: Timestamp,
    in_progress: Option<OngoingTrx>,
}

impl LeaseState {
    pub fn new(now: Timestamp, in_progress: Option<OngoingTrx>) -> Self {
        Self { now, in_progress }
    }
}

impl WithLease for LeaseState {
    type Output = StateResponse;

    type Error = ContractError;

    fn exec<Lpn, Asset, Lpp, Profit, Oracle>(
        self,
        lease: Lease<Lpn, Asset, Lpp, Profit, Oracle>,
    ) -> Result<Self::Output, Self::Error>
    where
        Lpn: Currency + Serialize,
        Lpp: LppLoanTrait<Lpn>,
        Oracle: OracleTrait<Lpn>,
        Profit: ProfitTrait,
        Asset: Currency + Serialize,
    {
        let state = lease.state(self.now)?;
        Ok(StateResponse::opened_from(state, self.in_progress))
    }
}
