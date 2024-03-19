use finance::{coin::Coin, duration::Duration, percent::Percent};
use sdk::cosmwasm_std::Timestamp;

use crate::finance::LpnCoin;

#[derive(Debug, PartialEq, Eq)]
pub struct State<Asset>
where
    Asset: ?Sized,
{
    pub amount: Coin<Asset>,
    pub interest_rate: Percent,
    pub interest_rate_margin: Percent,
    pub principal_due: LpnCoin,
    pub overdue_margin: LpnCoin,
    pub overdue_interest: LpnCoin,
    pub overdue_collect_in: Duration,
    pub due_margin: LpnCoin,
    pub due_interest: LpnCoin,
    pub validity: Timestamp,
}

impl<Asset> State<Asset> {
    pub fn total_due(&self) -> LpnCoin {
        self.principal_due
            + self.overdue_margin
            + self.overdue_interest
            + self.due_margin
            + self.due_interest
    }
}
