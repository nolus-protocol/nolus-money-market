use serde::{Deserialize, Serialize};

use oracle::stub::OracleRef;
use sdk::cosmwasm_std::Addr;
use timealarms::stub::TimeAlarmsRef;

use crate::{loan::LoanDTO, position::dto::PositionDTO};

#[derive(Serialize, Deserialize, Clone)]
#[cfg_attr(test, derive(Debug))]
pub struct LeaseDTO {
    pub(crate) addr: Addr,
    pub(crate) customer: Addr,
    pub(crate) position: PositionDTO,
    pub(crate) loan: LoanDTO,
    pub(crate) time_alarms: TimeAlarmsRef,
    pub(crate) oracle: OracleRef,
}

impl LeaseDTO {
    pub(crate) fn new(
        addr: Addr,
        customer: Addr,
        position: PositionDTO,
        loan: LoanDTO,
        time_alarms: TimeAlarmsRef,
        oracle: OracleRef,
    ) -> Self {
        Self {
            addr,
            customer,
            position,
            loan,
            time_alarms,
            oracle,
        }
    }
}
