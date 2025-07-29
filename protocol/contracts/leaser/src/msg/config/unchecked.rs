use finance::{duration::Duration, percent::Percent100};
use lease::api::{limits::MaxSlippages, open::PositionSpecDTO};
use serde::Deserialize;
#[cfg(feature = "internal.test.testing")]
use serde::Serialize;

use super::{NewConfig as ValidatedNewConfig, error::BrokenInvariant};

/// Bring invariant checking as a step in deserializing a PositionSpecDTO
#[derive(Deserialize)]
#[cfg_attr(feature = "internal.test.testing", derive(Serialize))]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub(super) struct NewConfig {
    lease_interest_rate_margin: Percent100,
    lease_position_spec: PositionSpecDTO,
    lease_due_period: Duration,
    lease_max_slippages: MaxSlippages,
}

impl TryFrom<NewConfig> for ValidatedNewConfig {
    type Error = BrokenInvariant<Self>;

    fn try_from(value: NewConfig) -> Result<Self, Self::Error> {
        let res = Self {
            lease_interest_rate_margin: value.lease_interest_rate_margin,
            lease_position_spec: value.lease_position_spec,
            lease_due_period: value.lease_due_period,
            lease_max_slippages: value.lease_max_slippages,
        };
        res.invariant_held().map(|_| res)
    }
}

#[cfg(all(feature = "internal.test.testing", test))]
mod test {
    use dex::MaxSlippage;
    use finance::{
        coin::{Amount, Coin, CoinDTO},
        duration::Duration,
        liability::Liability,
        percent::Percent100,
    };
    use lease::api::{limits::MaxSlippages, open::PositionSpecDTO};
    use platform::tests as platform_tests;

    use crate::{
        finance::{LpnCurrencies, LpnCurrency},
        msg::NewConfig as ValidatedConfig,
    };

    use super::NewConfig as NonvalidatedConfig;

    const DUE_PERIOD: Duration = Duration::from_nanos(604800000000000);
    const INTEREST_RATE_MARGIN: Percent100 = Percent100::from_permille(40);

    #[test]
    fn read_valid() {
        let spec = PositionSpecDTO::new(
            Liability::new(
                Percent100::from_percent(65),
                Percent100::from_percent(70),
                Percent100::from_percent(73),
                Percent100::from_percent(75),
                Percent100::from_percent(78),
                Percent100::from_percent(80),
                Duration::from_hours(1),
            ),
            lpn_coin_dto(1000),
            lpn_coin_dto(10),
        );

        let max_slippages = MaxSlippages {
            liquidation: MaxSlippage::unchecked(Percent100::from_permille(200)), // (100%-20%) of 10 LPN = 8 LPN != 0 LPN
        };

        assert_eq!(
            Ok(validated(spec, max_slippages)),
            platform_tests::ser_de(&NonvalidatedConfig {
                lease_interest_rate_margin: INTEREST_RATE_MARGIN,
                lease_position_spec: spec,
                lease_due_period: DUE_PERIOD,
                lease_max_slippages: max_slippages,
            })
        );
    }

    #[test]
    fn read_invalid() {
        let spec = PositionSpecDTO::new(
            Liability::new(
                Percent100::from_percent(65),
                Percent100::from_percent(70),
                Percent100::from_percent(73),
                Percent100::from_percent(75),
                Percent100::from_percent(78),
                Percent100::from_percent(80),
                Duration::from_hours(1),
            ),
            lpn_coin_dto(1000),
            lpn_coin_dto(10),
        );

        let max_slippages = MaxSlippages {
            liquidation: MaxSlippage::unchecked(Percent100::from_percent(91)), //(100%-91%) of 10 LPN = 0.9 LPN == 0 LPN
        };

        assert!(
            platform_tests::ser_de::<_, ValidatedConfig>(&NonvalidatedConfig {
                lease_interest_rate_margin: INTEREST_RATE_MARGIN,
                lease_position_spec: spec,
                lease_due_period: DUE_PERIOD,
                lease_max_slippages: max_slippages,
            })
            .is_err()
        );
    }

    fn validated(spec: PositionSpecDTO, max_slippages: MaxSlippages) -> ValidatedConfig {
        ValidatedConfig {
            lease_interest_rate_margin: INTEREST_RATE_MARGIN,
            lease_position_spec: spec,
            lease_due_period: DUE_PERIOD,
            lease_max_slippages: max_slippages,
        }
    }

    fn lpn_coin_dto(amount: Amount) -> CoinDTO<LpnCurrencies> {
        Coin::<LpnCurrency>::new(amount).into()
    }
}
