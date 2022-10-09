use cosmwasm_std::Addr;
use market_price_oracle::stub::OracleRef;
use serde::{Deserialize, Serialize};

use finance::{currency::SymbolOwned, liability::Liability};
use time_alarms::stub::TimeAlarmsRef;

use crate::loan::LoanDTO;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LeaseDTO {
    pub(crate) customer: Addr,
    pub(crate) currency: SymbolOwned,
    pub(crate) liability: Liability,
    pub(crate) loan: LoanDTO,
    pub(crate) time_alarms: TimeAlarmsRef,
    pub(crate) oracle: OracleRef,
}

impl LeaseDTO {
    pub(crate) fn new(
        customer: Addr,
        currency: SymbolOwned,
        liability: Liability,
        loan: LoanDTO,
        time_alarms: TimeAlarmsRef,
        oracle: OracleRef,
    ) -> Self {
        Self {
            customer,
            currency,
            liability,
            loan,
            time_alarms,
            oracle,
        }
    }
}
