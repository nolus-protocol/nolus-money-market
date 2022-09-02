use finance::{
    currency::Currency,
    percent::Percent,
    coin::Coin
};

pub(crate) struct Receipt<Lpn>
where
    Lpn: Currency,
{
    pub annual_interest_rate: Percent,
    pub borrowed: Coin<Lpn>,
}
