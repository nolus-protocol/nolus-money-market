use currencies::Lpn;
use dex::{ConnectionParams, Ics20Channel, MaxSlippage};
use finance::{
    coin::{Amount, Coin},
    duration::Duration,
    liability::Liability,
    percent::Percent,
};
use lease::api::{LpnCoinDTO, limits::MaxSlippages, open::PositionSpecDTO};
use platform::contract::Code;
use sdk::cosmwasm_std::Addr;

use crate::msg::{Config, InstantiateMsg, NewConfig};

mod contract_tests;

pub fn lpn_coin(amount: Amount) -> LpnCoinDTO {
    Coin::<Lpn>::from(amount).into()
}

pub fn config() -> Config {
    Config::new(Code::unchecked(10), dummy_instantiate_msg())
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
        lease_max_slippages: MaxSlippages {
            liquidation: MaxSlippage::unchecked(Percent::from_percent(13)),
        },
    }
}

fn dummy_instantiate_msg() -> InstantiateMsg {
    InstantiateMsg {
        lease_code: 10u16.into(),
        lpp: Addr::unchecked("LPP"),
        profit: Addr::unchecked("Profit"),
        reserve: Addr::unchecked("reserve"),
        time_alarms: Addr::unchecked("time alarms"),
        market_price_oracle: Addr::unchecked("oracle"),
        protocols_registry: Addr::unchecked("protocols"),
        lease_position_spec: PositionSpecDTO {
            liability: Liability::new(
                Percent::from_percent(10),
                Percent::from_percent(65),
                Percent::from_percent(72),
                Percent::from_percent(74),
                Percent::from_percent(76),
                Percent::from_percent(80),
                Duration::from_hours(12),
            ),
            min_asset: Coin::<Lpn>::from(120_000).into(),
            min_transaction: Coin::<Lpn>::from(12_000).into(),
        },
        lease_interest_rate_margin: Percent::from_percent(3),
        lease_due_period: Duration::from_days(14),
        lease_max_slippages: MaxSlippages {
            liquidation: MaxSlippage::unchecked(Percent::from_percent(20)),
        },
        lease_admin: Addr::unchecked("lease_admin_XYZ"),
        dex: ConnectionParams {
            connection_id: "conn-12".into(),
            transfer_channel: Ics20Channel {
                local_endpoint: "chan-1".into(),
                remote_endpoint: "chan-13".into(),
            },
        },
    }
}
