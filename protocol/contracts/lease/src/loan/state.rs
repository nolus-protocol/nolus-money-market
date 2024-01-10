use currency::Currency;
use finance::{coin::Coin, percent::Percent};

#[cfg_attr(test, derive(PartialEq, Eq, Debug))]
pub struct State<Lpn>
where
    Lpn: Currency,
{
    pub annual_interest: Percent,
    pub annual_interest_margin: Percent,
    pub principal_due: Coin<Lpn>,
    pub overdue_interest: Coin<Lpn>,
    pub overdue_margin_interest: Coin<Lpn>,
    pub due_interest: Coin<Lpn>,
    pub due_margin_interest: Coin<Lpn>,
}
