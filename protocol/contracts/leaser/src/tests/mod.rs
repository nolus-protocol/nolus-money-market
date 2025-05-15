use currencies::Lpn;
use finance::{
    coin::{Amount, Coin},
    duration::Duration,
    liability::Liability,
    percent::Percent,
};
use lease::api::{LpnCoinDTO, limits::MaxSlippage, open::PositionSpecDTO};

use crate::msg::NewConfig;

mod contract_tests;

pub fn lpn_coin(amount: Amount) -> LpnCoinDTO {
    Coin::<Lpn>::from(amount).into()
}

pub fn new_config() -> NewConfig {
    NewConfig {
        lease_interest_rate_margin: Percent::from_percent(5),
        lease_position_spec: PositionSpecDTO::new(
            Liability::new(
                Percent::from_percent(55),
                Percent::from_percent(60),
                Percent::from_percent(61),
                Percent::from_percent(62),
                Percent::from_percent(64),
                Percent::from_percent(65),
                Duration::from_hours(12),
            ),
            lpn_coin(4_211_442_000),
            lpn_coin(100_000),
        ),
        lease_due_period: Duration::from_secs(100),
        lease_max_slippage: MaxSlippage {
            liquidation: Percent::from_percent(13),
        },
    }
}
