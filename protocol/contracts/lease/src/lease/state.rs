use currency::Currency;
use finance::{coin::Coin, percent::Percent};
use sdk::cosmwasm_std::Timestamp;

#[derive(Debug, PartialEq, Eq)]
pub struct State<Asset, Lpn>
where
    Asset: Currency,
    Lpn: Currency,
{
    pub amount: Coin<Asset>,
    pub interest_rate: Percent,
    pub interest_rate_margin: Percent,
    pub principal_due: Coin<Lpn>,
    pub overdue_margin: Coin<Lpn>,
    pub overdue_interest: Coin<Lpn>,
    pub due_margin: Coin<Lpn>,
    pub due_interest: Coin<Lpn>,
    pub validity: Timestamp,
}
