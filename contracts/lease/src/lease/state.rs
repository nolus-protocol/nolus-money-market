use finance::{coin::Coin, currency::Currency, percent::Percent};
use sdk::cosmwasm_std::Timestamp;

#[derive(Debug, PartialEq, Eq)]
pub enum State<Asset, Lpn>
where
    Asset: Currency,
    Lpn: Currency,
{
    Opened {
        amount: Coin<Asset>,
        interest_rate: Percent,
        interest_rate_margin: Percent,
        principal_due: Coin<Lpn>,
        previous_margin_due: Coin<Lpn>,
        previous_interest_due: Coin<Lpn>,
        current_margin_due: Coin<Lpn>,
        current_interest_due: Coin<Lpn>,
        validity: Timestamp,
    },
    Paid(Coin<Asset>),
}
