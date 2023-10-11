use currency::lpn::Usdc;
use finance::{coin::Coin, liability::Liability};
use oracle::stub::OracleRef;
use serde::Deserialize;

use sdk::cosmwasm_std::Addr;
use timealarms::stub::TimeAlarmsRef;

use crate::{
    api::{LeaseCoin, PositionSpecDTO},
    loan::LoanDTO,
    position::PositionDTO,
};

use super::LeaseDTO as LeaseDTO_v6;

pub(crate) const MIN_ASSET: Coin<Usdc> = Coin::new(15_000_000);
pub(crate) const MIN_SELL_ASSET: Coin<Usdc> = Coin::new(10_000);

#[derive(Deserialize)]
pub(crate) struct LeaseDTO {
    pub(crate) addr: Addr,
    pub(crate) customer: Addr,
    pub(crate) amount: LeaseCoin,
    pub(crate) liability: Liability,
    pub(crate) loan: LoanDTO,
    pub(crate) time_alarms: TimeAlarmsRef,
    pub(crate) oracle: OracleRef,
}

impl LeaseDTO {
    pub(crate) fn migrate(self) -> LeaseDTO_v6 {
        LeaseDTO_v6::new(
            self.addr,
            self.customer,
            PositionDTO::new(
                self.amount,
                PositionSpecDTO::new_internal(
                    self.liability,
                    MIN_ASSET.into(),
                    MIN_SELL_ASSET.into(),
                ),
            ),
            self.loan,
            self.time_alarms,
            self.oracle,
        )
    }
}
