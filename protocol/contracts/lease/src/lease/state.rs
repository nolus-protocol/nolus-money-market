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
