use cosmwasm_std::Coin;
use finance::percent::Percent;

pub struct State {
    pub annual_interest: Percent,
    pub principal_due: Coin,
    pub interest_due: Coin,
}