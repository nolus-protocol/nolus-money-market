use finance::{coin::Coin, percent::Percent, currency::Currency};

pub struct State<Lpn>
where
    Lpn: Currency,
{
    pub annual_interest: Percent,
    pub principal_due: Coin<Lpn>,
    pub interest_due: Coin<Lpn>,
}
