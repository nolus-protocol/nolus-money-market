use cosmwasm_std::Addr;
use serde::{Deserialize, Serialize};

use finance::{currency::SymbolOwned, liability::Liability};

use crate::loan::LoanDTO;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LeaseDTO {
    pub(crate) customer: Addr,
    pub(crate) currency: SymbolOwned,
    pub(crate) liability: Liability,
    pub(crate) loan: LoanDTO,
    pub(crate) time_alarms: Addr,
    pub(crate) oracle: Addr,
}

impl LeaseDTO {
    pub(crate) fn new(
        customer: Addr,
        currency: SymbolOwned,
        liability: Liability,
        loan: LoanDTO,
        time_alarms: Addr,
        oracle: Addr,
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
