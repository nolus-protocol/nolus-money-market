use serde::{Deserialize, Serialize};

use finance::liability::dto::LiabilityDTO;
use oracle::stub::OracleRef;
use sdk::cosmwasm_std::Addr;
use timealarms::stub::TimeAlarmsRef;

use crate::{api::LeaseCoin, loan::LoanDTO};

#[derive(Serialize, Deserialize, Clone)]
#[cfg_attr(test, derive(Debug))]
pub struct LeaseDTO {
    pub(crate) addr: Addr,
    pub(crate) customer: Addr,
    pub(crate) amount: LeaseCoin,
    pub(crate) liability: LiabilityDTO,
    pub(crate) loan: LoanDTO,
    pub(crate) time_alarms: TimeAlarmsRef,
    pub(crate) oracle: OracleRef,
}

impl LeaseDTO {
    pub(crate) fn new(
        addr: Addr,
        customer: Addr,
        amount: LeaseCoin,
        liability: LiabilityDTO,
        loan: LoanDTO,
        time_alarms: TimeAlarmsRef,
        oracle: OracleRef,
    ) -> Self {
        Self {
            addr,
            customer,
            amount,
            liability,
            loan,
            time_alarms,
            oracle,
        }
    }
}
