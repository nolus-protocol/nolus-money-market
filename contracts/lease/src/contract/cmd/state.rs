use serde::Serialize;

use finance::currency::Currency;
use lpp::stub::lender::LppLender as LppLenderTrait;
use oracle::stub::Oracle as OracleTrait;
use profit::stub::Profit as ProfitTrait;
use sdk::cosmwasm_std::{to_binary, Binary, Timestamp};
use timealarms::stub::TimeAlarms as TimeAlarmsTrait;

use crate::{
    api::StateResponse,
    error::ContractError,
    lease::{with_lease::WithLease, Lease},
};

pub struct LeaseState {
    now: Timestamp,
}

impl LeaseState {
    pub fn new(now: Timestamp) -> Self {
        Self { now }
    }
}

impl WithLease for LeaseState {
    type Output = Binary;

    type Error = ContractError;

    fn exec<Lpn, Asset, Lpp, Profit, TimeAlarms, Oracle>(
        self,
        lease: Lease<Lpn, Asset, Lpp, Profit, TimeAlarms, Oracle>,
    ) -> Result<Self::Output, Self::Error>
    where
        Lpn: Currency + Serialize,
        Lpp: LppLenderTrait<Lpn>,
        TimeAlarms: TimeAlarmsTrait,
        Oracle: OracleTrait<Lpn>,
        Profit: ProfitTrait,
        Asset: Currency + Serialize,
    {
        let resp: StateResponse = lease.state(self.now)?.into();
        to_binary(&resp).map_err(ContractError::from)
    }
}
