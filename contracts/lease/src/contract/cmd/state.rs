use cosmwasm_std::{to_binary, Binary, Timestamp};
use serde::Serialize;

use finance::currency::Currency;
use lpp::stub::lender::LppLender as LppLenderTrait;
use market_price_oracle::stub::Oracle as OracleTrait;
use profit::stub::Profit as ProfitTrait;
use time_alarms::stub::TimeAlarms as TimeAlarmsTrait;

use crate::{
    error::ContractError,
    lease::{Lease, WithLease},
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
        let resp = lease.state(self.now)?;
        to_binary(&resp).map_err(ContractError::from)
    }
}
