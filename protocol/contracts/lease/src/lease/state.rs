use finance::{coin::Coin, duration::Duration, percent::Percent};
use sdk::cosmwasm_std::Timestamp;

#[derive(Debug, PartialEq, Eq)]
pub struct State<Asset, Lpn>
where
    Asset: ?Sized,
    Lpn: ?Sized,
{
    pub amount: Coin<Asset>,
    pub interest_rate: Percent,
    pub interest_rate_margin: Percent,
    pub principal_due: Coin<Lpn>,
    pub overdue_margin: Coin<Lpn>,
    pub overdue_interest: Coin<Lpn>,
    pub overdue_collect_in: Duration,
    pub due_margin: Coin<Lpn>,
    pub due_interest: Coin<Lpn>,
    pub validity: Timestamp,
}

impl<Asset, Lpn> State<Asset, Lpn>
where
    Lpn: ?Sized,
{
    pub fn total_due(&self) -> Coin<Lpn> {
        self.principal_due
            + self.overdue_margin
            + self.overdue_interest
            + self.due_margin
            + self.due_interest
    }
}
