use serde::{Deserialize, Serialize};

use finance::{coin::CoinDTO, liability::Liability};
use market_price_oracle::stub::OracleRef;
use sdk::cosmwasm_std::Addr;
use time_alarms::stub::TimeAlarmsRef;

use crate::loan::LoanDTO;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LeaseDTO {
    pub(crate) customer: Addr,
    pub(crate) amount: CoinDTO,
    pub(crate) liability: Liability,
    pub(crate) loan: LoanDTO,
    pub(crate) time_alarms: TimeAlarmsRef,
    pub(crate) oracle: OracleRef,
}

impl LeaseDTO {
    pub(crate) fn new(
        customer: Addr,
        amount: CoinDTO,
        liability: Liability,
        loan: LoanDTO,
        time_alarms: TimeAlarmsRef,
        oracle: OracleRef,
    ) -> Self {
        Self {
            customer,
            amount,
            liability,
            loan,
            time_alarms,
            oracle,
        }
    }
}
