use finance::{coin::Coin, currency::Currency, percent::Percent};

#[cfg_attr(test, derive(PartialEq, Eq, Debug))]
pub struct State<Lpn>
where
    Lpn: Currency,
{
    pub annual_interest: Percent,
    pub annual_interest_margin: Percent,
    pub principal_due: Coin<Lpn>,
    pub previous_interest_due: Coin<Lpn>,
    pub current_interest_due: Coin<Lpn>,
    pub previous_margin_interest_due: Coin<Lpn>,
    pub current_margin_interest_due: Coin<Lpn>,
}
