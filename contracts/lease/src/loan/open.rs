use finance::{coin::Coin, currency::Currency, percent::Percent};

pub(crate) struct Receipt<Lpn>
where
    Lpn: Currency,
{
    pub annual_interest_rate: Percent,
    pub borrowed: Coin<Lpn>,
}
