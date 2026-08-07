use serde::{Deserialize, Serialize};

use currency::CurrencyDef;
use error::BrokenInvariant;
use finance::{coin::Coin, duration::Duration, percent::Percent100};
use lease::api::{limits::MaxSlippages, open::PositionSpecDTO};

use crate::finance::LpnCurrency;

mod error;

/// The modifiable part of the leaser configuration
#[derive(Serialize, Deserialize, Clone, PartialEq, Eq)]
#[cfg_attr(any(test, feature = "testing"), derive(Debug))]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct LeaseConfig {
    pub interest_rate_margin: Percent100,
    pub position_spec: PositionSpecDTO,
    pub due_period: Duration,
    pub max_slippages: MaxSlippages,
}

/// A [LeaseConfig] coming from an external, untrusted source
///
/// Assume the data may be wrong and check its invariant as a deserialization
/// step, so any config supplied over the wire, at instantiation or later, is
/// valid by construction.
#[derive(Serialize, Deserialize, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "testing", derive(Debug))]
#[serde(try_from = "LeaseConfig", into = "LeaseConfig")]
pub struct LeaseConfigExternal(LeaseConfig);

impl TryFrom<LeaseConfig> for LeaseConfigExternal {
    type Error = BrokenInvariant<LeaseConfig>;

    fn try_from(config: LeaseConfig) -> Result<Self, Self::Error> {
        config.invariant_held().map(|()| Self(config))
    }
}

impl From<LeaseConfigExternal> for LeaseConfig {
    fn from(checked: LeaseConfigExternal) -> Self {
        checked.0
    }
}

impl LeaseConfig {
    fn invariant_held(&self) -> Result<(), BrokenInvariant<Self>> {
        let min_transaction: Coin<LpnCurrency> = self
            .position_spec
            .min_transaction
            .as_specific(LpnCurrency::dto());
        let MaxSlippages {
            open,
            repay,
            close,
            liquidation,
        } = self.max_slippages;
        BrokenInvariant::r#if(
            [open, repay, close, liquidation]
                .into_iter()
                .any(|max_slippage| max_slippage.min_out(min_transaction).is_zero()),
            "The min output from a dex transaction of the min transaction amount should be positive",
        )
    }
}

#[cfg(all(feature = "internal.test.testing", test))]
mod test {
    use dex::MaxSlippage;
    use finance::{duration::Duration, liability::Liability, percent::Percent100};
    use lease::api::{limits::MaxSlippages, open::PositionSpecDTO};
    use platform::tests as platform_tests;

    use crate::tests;

    use super::{LeaseConfig, LeaseConfigExternal};

    const DUE_PERIOD: Duration = Duration::from_nanos(604800000000000);
    const INTEREST_RATE_MARGIN: Percent100 = Percent100::from_permille(40);
    // (100% - 91%) of 10 LPN = 0.9 LPN == 0 LPN
    const INVALID_SLIPPAGE: MaxSlippage = MaxSlippage::unchecked(Percent100::from_percent(91));

    #[test]
    fn read_valid() {
        let config = valid_lease_config();

        assert_eq!(
            LeaseConfigExternal::try_from(config.clone()).unwrap(),
            platform_tests::ser_de(&config).unwrap()
        );
    }

    #[test]
    fn read_invalid() {
        let broken_per_field = [
            MaxSlippages {
                open: INVALID_SLIPPAGE,
                ..valid_max_slippages()
            },
            MaxSlippages {
                repay: INVALID_SLIPPAGE,
                ..valid_max_slippages()
            },
            MaxSlippages {
                close: INVALID_SLIPPAGE,
                ..valid_max_slippages()
            },
            MaxSlippages {
                liquidation: INVALID_SLIPPAGE,
                ..valid_max_slippages()
            },
        ];

        for max_slippages in broken_per_field {
            let config = LeaseConfig {
                max_slippages,
                ..valid_lease_config()
            };

            assert!(platform_tests::ser_de::<_, LeaseConfigExternal>(&config).is_err());
        }
    }

    fn valid_lease_config() -> LeaseConfig {
        LeaseConfig {
            interest_rate_margin: INTEREST_RATE_MARGIN,
            position_spec: PositionSpecDTO::new(
                Liability::new(
                    Percent100::from_percent(65),
                    Percent100::from_percent(70),
                    Percent100::from_percent(73),
                    Percent100::from_percent(75),
                    Percent100::from_percent(78),
                    Percent100::from_percent(80),
                    Duration::from_hours(1),
                ),
                tests::lpn_coin_dto(1000),
                tests::lpn_coin_dto(10),
            ),
            due_period: DUE_PERIOD,
            max_slippages: valid_max_slippages(),
        }
    }

    // each field yields a non-zero min output on the 10 LPN min transaction
    fn valid_max_slippages() -> MaxSlippages {
        MaxSlippages {
            open: MaxSlippage::unchecked(Percent100::from_permille(170)),
            repay: MaxSlippage::unchecked(Percent100::from_permille(180)),
            close: MaxSlippage::unchecked(Percent100::from_permille(190)),
            liquidation: MaxSlippage::unchecked(Percent100::from_permille(200)),
        }
    }
}
