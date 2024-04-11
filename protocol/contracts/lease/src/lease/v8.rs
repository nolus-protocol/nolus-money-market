use oracle::stub::OracleRef;
use serde::Deserialize;

use sdk::cosmwasm_std::Addr;
use timealarms::stub::TimeAlarmsRef;

use crate::{finance::ReserveRef, loan::LoanDTO, position::PositionDTO};

use super::LeaseDTO as LeaseDTO_v9;

#[derive(Deserialize)]
pub(crate) struct LeaseDTO {
    pub(crate) addr: Addr,
    pub(crate) customer: Addr,
    pub(crate) position: PositionDTO,
    pub(crate) loan: LoanDTO,
    pub(crate) time_alarms: TimeAlarmsRef,
    pub(crate) oracle: OracleRef,
}

impl LeaseDTO {
    pub(crate) fn migrate(self, reserve: ReserveRef) -> LeaseDTO_v9 {
        LeaseDTO_v9::new(
            self.addr,
            self.customer,
            self.position,
            self.loan,
            self.time_alarms,
            self.oracle,
            reserve,
        )
    }
}
