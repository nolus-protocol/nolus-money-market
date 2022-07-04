use finance::{percent::Percent, coin::Coin};

pub struct State<Lpn> {
    pub annual_interest: Percent,
    pub principal_due: Coin<Lpn>,
    pub interest_due: Coin<Lpn>,
}
