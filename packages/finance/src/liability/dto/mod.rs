use serde::{Deserialize, Serialize};

use sdk::schemars::{self, JsonSchema};

use crate::{
    coin::LpnCoin,
    duration::Duration,
    error::{Error, Result},
    percent::Percent,
};
use currency::Currency;

use super::Liability;

mod unchecked;

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
#[serde(try_from = "unchecked::LiabilityDTO")]
pub struct LiabilityDTO {
    initial: Percent,
    /// The healty percentage of the amount due versus the locked collateral
    /// healthy >= initial
    healthy: Percent,
    /// The percentage above which the first liquidity warning is issued.
    first_liq_warn: Percent,
    /// The percentage above which the second liquidity warning is issued.
    second_liq_warn: Percent,
    /// The percentage above which the third liquidity warning is issued.
    third_liq_warn: Percent,
    /// The maximum percentage of the amount due versus the locked collateral
    /// max > healthy
    max: Percent,
    /// The minimum amount that triggers a liquidation
    min_liquidation: LpnCoin,
    ///  The minimum amount that a lease asset should be evaluated past any partial liquidation. If not, a full liquidation is performed
    min_asset: LpnCoin,
    /// At what time cadence to recalculate the liability
    ///
    /// Limitation: recalc_time >= 1 hour
    recalc_time: Duration,
}

impl LiabilityDTO {
    #[track_caller]
    #[cfg(any(test, feature = "testing"))]
    pub fn new(
        initial: Percent,
        delta_to_healthy: Percent,
        delta_to_max: Percent,
        minus_delta_of_liq_warns: (Percent, Percent, Percent),
        min_liquidation: LpnCoin,
        min_asset: LpnCoin,
        recalc_time: Duration,
    ) -> Self {
        let healthy = initial + delta_to_healthy;
        let max = healthy + delta_to_max;
        let third_liquidity_warning = max - minus_delta_of_liq_warns.2;
        let second_liquidity_warning = third_liquidity_warning - minus_delta_of_liq_warns.1;
        let first_liquidity_warning = second_liquidity_warning - minus_delta_of_liq_warns.0;
        let obj = Self {
            initial,
            healthy,
            max,
            first_liq_warn: first_liquidity_warning,
            second_liq_warn: second_liquidity_warning,
            third_liq_warn: third_liquidity_warning,
            min_liquidation,
            min_asset,
            recalc_time,
        };
        debug_assert_eq!(Ok(()), obj.invariant_held());
        obj
    }

    fn invariant_held(&self) -> Result<()> {
        check(self.initial > Percent::ZERO, "Initial % should not be zero")?;

        check(
            self.initial <= self.healthy,
            "Initial % should be <= healthy %",
        )?;

        check(
            self.healthy < self.first_liq_warn,
            "Healthy % should be < first liquidation %",
        )?;
        check(
            self.first_liq_warn < self.second_liq_warn,
            "First liquidation % should be < second liquidation %",
        )?;
        check(
            self.second_liq_warn < self.third_liq_warn,
            "Second liquidation % should be < third liquidation %",
        )?;
        check(
            self.third_liq_warn < self.max,
            "Third liquidation % should be < max %",
        )?;
        check(self.max <= Percent::HUNDRED, "Max % should be <= 100%")?;
        check(
            self.recalc_time >= Duration::HOUR,
            "Recalculation cadence should be >= 1h",
        )?;

        Ok(())
    }
}

fn check(invariant: bool, msg: &str) -> Result<()> {
    Error::broken_invariant_if::<LiabilityDTO>(!invariant, msg)
}

impl<Lpn> TryFrom<LiabilityDTO> for Liability<Lpn>
where
    Lpn: Currency,
{
    type Error = Error;

    fn try_from(dto: LiabilityDTO) -> Result<Self> {
        Ok(Self {
            initial: dto.initial,
            healthy: dto.healthy,
            first_liq_warn: dto.first_liq_warn,
            second_liq_warn: dto.second_liq_warn,
            third_liq_warn: dto.third_liq_warn,
            max: dto.max,
            min_liquidation: dto.min_liquidation.try_into()?,
            min_asset: dto.min_asset.try_into()?,
            recalc_time: dto.recalc_time,
        })
    }
}

impl<Lpn> From<Liability<Lpn>> for LiabilityDTO
where
    Lpn: Currency,
{
    fn from(value: Liability<Lpn>) -> Self {
        Self {
            initial: value.initial,
            healthy: value.healthy,
            first_liq_warn: value.first_liq_warn,
            second_liq_warn: value.second_liq_warn,
            third_liq_warn: value.third_liq_warn,
            max: value.max,
            min_liquidation: value.min_liquidation.into(),
            min_asset: value.min_asset.into(),
            recalc_time: value.recalc_time,
        }
    }
}

#[cfg(test)]
mod test {
    use currency::lpn::Usdc;
    use sdk::cosmwasm_std::{from_slice, StdError};

    use crate::{
        coin::Coin,
        duration::Duration,
        liability::{LiabilityDTO, MIN_ASSET_AMOUNT, MIN_LIQ_AMOUNT},
        percent::Percent,
    };

    type CoinLpn = Coin<Usdc>;

    const MIN_LIQUIDATION: CoinLpn = CoinLpn::new(MIN_LIQ_AMOUNT);
    const MIN_ASSET: CoinLpn = CoinLpn::new(MIN_ASSET_AMOUNT);

    #[test]
    fn new_valid() {
        let exp = LiabilityDTO {
            initial: Percent::from_percent(10),
            healthy: Percent::from_percent(10),
            first_liq_warn: Percent::from_percent(12),
            second_liq_warn: Percent::from_percent(13),
            third_liq_warn: Percent::from_percent(14),
            max: Percent::from_percent(15),
            min_liquidation: MIN_LIQUIDATION.into(),
            min_asset: MIN_ASSET.into(),
            recalc_time: Duration::from_hours(10),
        };

        assert_load_ok(br#"{"initial":100,"healthy":100,"first_liq_warn":120,"second_liq_warn":130,"third_liq_warn":140,"max":150,"min_liquidation": {"amount": "10000", "ticker": "USDC"},"min_asset": {"amount": "15000000", "ticker": "USDC"},"recalc_time": 36000000000000}"#,
        exp);
    }

    #[test]
    fn new_edge_case() {
        let exp = LiabilityDTO {
            initial: Percent::from_percent(1),
            healthy: Percent::from_percent(1),
            first_liq_warn: Percent::from_permille(11),
            second_liq_warn: Percent::from_permille(12),
            third_liq_warn: Percent::from_permille(13),
            max: Percent::from_permille(14),
            min_liquidation: MIN_LIQUIDATION.into(),
            min_asset: MIN_ASSET.into(),
            recalc_time: Duration::HOUR,
        };

        assert_load_ok(br#"{"initial":10,"healthy":10,"first_liq_warn":11,"second_liq_warn":12,"third_liq_warn":13,
                        "max":14,"min_liquidation": {"amount": "10000", "ticker": "USDC"},"min_asset": {"amount": "15000000", "ticker": "USDC"},"recalc_time":3600000000000}"#, exp);
    }

    #[test]
    fn new_invalid_init_percent() {
        assert_load_err(br#"{"initial":0,"healthy":10,"first_liq_warn":11,"second_liq_warn":12,"third_liq_warn":13,
                        "max":14,"min_liquidation": {"amount": "10000", "ticker": "USDC"},"min_asset": {"amount": "15000000", "ticker": "USDC"},"recalc_time":3600000000000}"#, "should not be zero");
    }

    #[test]
    fn new_overflow_percent() {
        const ERR_MSG: &str = "Invalid number";

        assert_load_err(br#"{"initial":4294967296,"healthy":10,"first_liq_warn":11,"second_liq_warn":12,"third_liq_warn":13,
                        "max":14,"min_liquidation": {"amount": "10000", "ticker": "USDC"},"min_asset": {"amount": "15000000", "ticker": "USDC"},"recalc_time":3600000000000}"#, "Invalid number"); // u32::MAX + 1

        assert_load_err(br#"{"initial":10,"healthy":4294967296,"first_liq_warn":11,"second_liq_warn":12,"third_liq_warn":13,
                        "max":14,"min_liquidation": {"amount": "10000", "ticker": "USDC"},"min_asset": {"amount": "15000000", "ticker": "USDC"},"recalc_time":3600000000000}"#, ERR_MSG); // u32::MAX + 1

        assert_load_err(br#"{"initial":10,"healthy":10,"first_liq_warn":4294967296,"second_liq_warn":12,"third_liq_warn":13,
                        "max":14,"min_liquidation": {"amount": "10000", "ticker": "USDC"},"min_asset": {"amount": "15000000", "ticker": "USDC"},"recalc_time":3600000000000}"#, ERR_MSG); // u32::MAX + 1

        assert_load_err(br#"{"initial":10,"healthy":10,"first_liq_warn":11,"second_liq_warn":4294967296,"third_liq_warn":13,
                        "max":14,"min_liquidation": {"amount": "10000", "ticker": "USDC"},"min_asset": {"amount": "15000000", "ticker": "USDC"},"recalc_time":3600000000000}"#, ERR_MSG); // u32::MAX + 1

        assert_load_err(br#"{"initial":10,"healthy":10,"first_liq_warn":11,"second_liq_warn":12,"third_liq_warn":4294967296,
                        "max":14,"min_liquidation": {"amount": "10000", "ticker": "USDC"},"min_asset": {"amount": "15000000", "ticker": "USDC"},"recalc_time":3600000000000}"#, ERR_MSG); // u32::MAX + 1

        assert_load_err(br#"{"initial":10,"healthy":10,"first_liq_warn":11,"second_liq_warn":12,"third_liq_warn":13,
                        "max":4294967296,"min_liquidation": {"amount": "10000", "ticker": "USDC"},"min_asset": {"amount": "15000000", "ticker": "USDC"},"recalc_time":3600000000000}"#, ERR_MSG); // u32::MAX + 1

        assert_load_err(br#"{"initial":10,"healthy":10,"first_liq_warn":11,"second_liq_warn":12,"third_liq_warn":13,
                        "max":14,"min_liquidation": {"amount": "10000", "ticker": "USDC"},"min_asset": {"amount": "15000000", "ticker": "USDC"},"recalc_time":18446744073709551616}"#, ERR_MSG);
        // u64::MAX + 1
    }

    #[test]
    fn new_invalid_percents_relations() {
        assert_load_err(br#"{"initial":10,"healthy":9,"first_liq_warn":11,"second_liq_warn":12,"third_liq_warn":13,
                        "max":14,"min_liquidation": {"amount": "10000", "ticker": "USDC"},"min_asset": {"amount": "15000000", "ticker": "USDC"},"recalc_time":3600000000000}"#, "<= healthy %");
        assert_load_err(br#"{"initial":10,"healthy":10,"first_liq_warn":10,"second_liq_warn":12,"third_liq_warn":13,
                        "max":14,"min_liquidation": {"amount": "10000", "ticker": "USDC"},"min_asset": {"amount": "15000000", "ticker": "USDC"},"recalc_time":3600000000000}"#, "< first liquidation %");
        assert_load_err(br#"{"initial":10,"healthy":10,"first_liq_warn":11,"second_liq_warn":11,"third_liq_warn":13,
                        "max":14,"min_liquidation": {"amount": "10000", "ticker": "USDC"},"min_asset": {"amount": "15000000", "ticker": "USDC"},"recalc_time":3600000000000}"#, "< second liquidation %");
        assert_load_err(br#"{"initial":10,"healthy":10,"first_liq_warn":11,"second_liq_warn":12,"third_liq_warn":12,
                        "max":14,"min_liquidation": {"amount": "10000", "ticker": "USDC"},"min_asset": {"amount": "15000000", "ticker": "USDC"},"recalc_time":3600000000000}"#, "< third liquidation %");
        assert_load_err(br#"{"initial":10,"healthy":10,"first_liq_warn":11,"second_liq_warn":12,"third_liq_warn":13,
                        "max":13,"min_liquidation": {"amount": "10000", "ticker": "USDC"},"min_asset": {"amount": "15000000", "ticker": "USDC"},"recalc_time":3600000000000}"#, "< max %");
    }

    #[test]
    fn new_invalid_recalc_hours() {
        assert_load_err(br#"{"initial":10,"healthy":10,"first_liq_warn":11,"second_liq_warn":12,"third_liq_warn":13,
                        "max":14,"min_liquidation": {"amount": "10000", "ticker": "USDC"},"min_asset": {"amount": "15000000", "ticker": "USDC"},"recalc_time":3599999999999}"#, ">= 1h");
    }

    fn assert_load_ok(json: &[u8], exp: LiabilityDTO) {
        assert_eq!(Ok(exp), from_slice::<LiabilityDTO>(json));
    }

    #[track_caller]
    fn assert_load_err(json: &[u8], msg: &str) {
        assert!(matches!(
            from_slice::<LiabilityDTO>(json),
            Err(StdError::ParseErr {
                target_type,
                msg: real_msg
            }) if target_type.contains("Liability") && real_msg.contains(msg)
        ));
    }
}
