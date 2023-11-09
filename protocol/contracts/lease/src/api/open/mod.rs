#[cfg(feature = "skel")]
use serde::Deserialize;
use serde::Serialize;

use currency::SymbolOwned;
pub use dex::{ConnectionParams, Ics20Channel};
use finance::{duration::Duration, liability::Liability, percent::Percent};
use sdk::{
    cosmwasm_std::Addr,
    schemars::{self, JsonSchema},
};

#[cfg(feature = "skel")]
use crate::{error::ContractError, error::ContractResult};

use super::LpnCoin;

#[cfg(feature = "skel")]
mod unchecked;

#[derive(Serialize, Clone, PartialEq, Eq, JsonSchema)]
#[cfg_attr(feature = "skel", derive(Deserialize))]
#[cfg_attr(any(test, feature = "testing"), derive(Debug))]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct NewLeaseContract {
    /// An application form for opening a new lease
    pub form: NewLeaseForm,
    /// Connection parameters of a Dex capable to perform currency swaps
    pub dex: ConnectionParams,
    /// A contract to be notified when a lease just went into a final state
    ///
    /// The finalizer API should provide all `FinalizerExecuteMsg` variants.
    pub finalizer: Addr,
}

#[derive(Serialize, Clone, PartialEq, Eq, JsonSchema)]
#[cfg_attr(feature = "skel", derive(Deserialize))]
#[cfg_attr(any(test, feature = "testing"), derive(Debug))]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct NewLeaseForm {
    /// The customer who wants to open a lease.
    pub customer: Addr,
    /// Ticker of the currency this lease will be about.
    pub currency: SymbolOwned,
    /// Maximum Loan-to-Downpayment percentage of the new lease, optional.
    pub max_ltd: Option<Percent>,
    /// Position parameters
    pub position_spec: PositionSpecDTO,
    /// Loan parameters
    pub loan: LoanForm,
    /// The time alarms contract the lease uses to get time notifications
    pub time_alarms: Addr,
    /// The oracle contract that sends market price alerts to the lease
    pub market_price_oracle: Addr,
}

#[derive(Serialize, Clone, PartialEq, Eq, JsonSchema)]
#[cfg_attr(feature = "skel", derive(Deserialize))]
#[cfg_attr(any(test, feature = "testing"), derive(Debug))]
#[serde(deny_unknown_fields, rename = "loan", rename_all = "snake_case")]
/// The value remains intact.
pub struct LoanForm {
    /// The delta added on top of the LPP Loan interest rate.
    ///
    /// The amount, a part of any payment, goes to the Profit contract.
    pub annual_margin_interest: Percent,
    /// The Liquidity Provider Pool, LPP, that lends the necessary amount for this lease.
    pub lpp: Addr,
    /// Interest repayment parameters
    pub interest_payment: InterestPaymentSpec,
    /// The Profit contract to which the margin interest is sent.
    pub profit: Addr,
}

#[derive(Serialize, Clone, PartialEq, Eq, JsonSchema)]
#[cfg_attr(
    feature = "skel",
    derive(Deserialize),
    serde(deny_unknown_fields, try_from = "unchecked::InterestPaymentSpec")
)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(any(test, feature = "testing"), derive(Debug))]
pub struct InterestPaymentSpec {
    /// How long is a period for which the interest is due
    due_period: Duration,
    /// How long after the due period ends the interest may be paid before initiating a liquidation
    grace_period: Duration,
}

#[cfg(feature = "skel")]
impl InterestPaymentSpec {
    #[cfg(any(test, feature = "testing"))]
    pub fn new(due_period: Duration, grace_period: Duration) -> Self {
        let res = Self {
            due_period,
            grace_period,
        };
        debug_assert_eq!(Ok(()), res.invariant_held());
        res
    }

    pub fn grace_period(&self) -> Duration {
        self.grace_period
    }

    pub fn due_period(&self) -> Duration {
        self.due_period
    }

    fn invariant_held(&self) -> ContractResult<()> {
        ContractError::broken_invariant_if::<InterestPaymentSpec>(
            self.due_period == Duration::default(),
            "The interest due period should be with non-zero length",
        )
        .and_then(|_| {
            ContractError::broken_invariant_if::<InterestPaymentSpec>(
                self.due_period <= self.grace_period,
                "The interest due period should be longer than grace period to avoid overlapping",
            )
        })
    }
}

#[derive(Serialize, Clone, PartialEq, Eq, JsonSchema)]
#[cfg_attr(
    feature = "skel",
    derive(Deserialize),
    serde(deny_unknown_fields, try_from = "unchecked::PositionSpecDTO")
)]
#[cfg_attr(any(test, feature = "testing"), derive(Debug))]
#[serde(rename_all = "snake_case")]
pub struct PositionSpecDTO {
    /// Liability constraints
    pub liability: Liability,
    ///  The minimum amount that a lease asset should be evaluated past any
    ///  partial liquidation or close. If not, a full liquidation is performed
    pub min_asset: LpnCoin,
    /// The minimum amount to liquidate or close. Any attempt to liquidate a smaller
    /// amount would be postponed until the amount goes above this limit
    //TODO: rename it to 'min_transaction' in the next migration
    pub min_sell_asset: LpnCoin,
}

#[cfg(feature = "skel")]
impl PositionSpecDTO {
    #[cfg(any(test, feature = "testing", feature = "osmosis", feature = "migration"))]
    pub(crate) fn new_internal(
        liability: Liability,
        min_asset: LpnCoin,
        min_sell_asset: LpnCoin,
    ) -> Self {
        let obj = Self {
            liability,
            min_asset,
            min_sell_asset,
        };
        debug_assert_eq!(Ok(()), obj.invariant_held());
        obj
    }

    #[cfg(any(test, feature = "testing", feature = "migration"))]
    pub fn new(liability: Liability, min_asset: LpnCoin, min_sell_asset: LpnCoin) -> Self {
        let obj = Self::new_internal(liability, min_asset, min_sell_asset);
        obj.invariant_held().expect("Leaser invariant to be held");
        obj
    }

    fn invariant_held(&self) -> ContractResult<()> {
        Self::check(
            !self.min_asset.is_zero(),
            "Min asset amount should be positive",
        )
        .and(Self::check(
            !self.min_sell_asset.is_zero(),
            "Min sell asset amount should be positive",
        ))
        .and(Self::check(
            self.min_asset.ticker() == self.min_sell_asset.ticker(),
            "The ticker of min asset should be the same as the ticker of min sell asset",
        ))
    }

    fn check(invariant: bool, msg: &str) -> ContractResult<()> {
        ContractError::broken_invariant_if::<Self>(!invariant, msg)
    }
}

#[cfg(all(test, feature = "skel"))]
mod test_invariant {
    use finance::duration::Duration;
    use sdk::cosmwasm_std::{from_json, StdError};

    use super::InterestPaymentSpec;

    #[test]
    #[should_panic = "non-zero length"]
    fn due_period_zero() {
        new_invalid(Duration::default(), Duration::from_hours(2));
    }

    #[test]
    fn due_period_zero_json() {
        let r = from_json(br#"{"due_period": 0, "grace_period": 10}"#);
        assert_err(r, "non-zero length");
    }

    #[test]
    #[should_panic = "should be longer than"]
    fn due_shorter_than_grace() {
        new_invalid(
            Duration::from_days(100),
            Duration::from_days(100) + Duration::from_nanos(1),
        );
    }

    #[test]
    fn due_shorter_than_grace_json() {
        let r = from_json(br#"{"due_period": 9, "grace_period": 10}"#);
        assert_err(r, "should be longer than");
    }

    #[test]
    #[should_panic = "should be longer than"]
    fn due_equal_to_grace() {
        new_invalid(Duration::from_days(100), Duration::from_days(100));
    }

    #[test]
    fn due_equal_to_grace_json() {
        let r = from_json(br#"{"due_period": 10, "grace_period": 10}"#);
        assert_err(r, "should be longer than");
    }

    fn new_invalid(due_period: Duration, grace_period: Duration) {
        let _p = InterestPaymentSpec::new(due_period, grace_period);
        #[cfg(not(debug_assertions))]
        {
            _p.invariant_held().expect("should have returned an error");
        }
    }

    fn assert_err(r: Result<InterestPaymentSpec, StdError>, msg: &str) {
        assert!(matches!(
            r,
            Err(StdError::ParseErr {
                target_type,
                msg: real_msg
            }) if target_type.contains("InterestPaymentSpec") && real_msg.contains(msg)
        ));
    }
}

#[cfg(all(test, feature = "skel"))]
mod test_position_spec {
    use currency::dex::test::StableC1;
    use finance::{coin::Coin, duration::Duration, liability::Liability, percent::Percent};
    use sdk::cosmwasm_std::{from_json, StdError};

    use super::PositionSpecDTO;

    type LpnCoin = Coin<StableC1>;

    #[test]
    fn new_valid() {
        let liability = Liability::new(
            Percent::from_percent(65),
            Percent::from_percent(5),
            Percent::from_percent(10),
            Percent::from_percent(2),
            Percent::from_percent(3),
            Percent::from_percent(2),
            Duration::from_hours(1),
        );
        let position_spec = PositionSpecDTO::new(
            liability,
            LpnCoin::new(9000000).into(),
            LpnCoin::new(5000).into(),
        );

        assert_load_ok(position_spec, br#"{"liability":{"initial":650,"healthy":700,"first_liq_warn":730,"second_liq_warn":750,"third_liq_warn":780,"max":800,"recalc_time":3600000000000},"min_asset":{"amount":"9000000","ticker":"USDC"},"min_sell_asset":{"amount":"5000","ticker":"USDC"}}"#);
    }

    #[test]
    fn zero_min_asset() {
        let r = from_json(br#"{"liability":{"initial":650,"healthy":700,"first_liq_warn":730,"second_liq_warn":750,"third_liq_warn":780,"max":800,"recalc_time":3600000000000},"min_asset":{"amount":"0","ticker":"USDC"},"min_sell_asset":{"amount":"5000","ticker":"USDC"}}"#);
        assert_err(r, "should be positive");
    }

    #[test]
    fn zero_min_sell_asset() {
        let r = from_json(br#"{"liability":{"initial":650,"healthy":700,"first_liq_warn":730,"second_liq_warn":750,"third_liq_warn":780,"max":800,"recalc_time":3600000000000},"min_asset":{"amount":"9000000","ticker":"USDC"},"min_sell_asset":{"amount":"0","ticker":"USDC"}}"#);
        assert_err(r, "should be positive");
    }

    #[test]
    fn invalid_ticker() {
        let r = from_json(br#"{"liability":{"initial":650,"healthy":700,"first_liq_warn":730,"second_liq_warn":750,"third_liq_warn":780,"max":800,"recalc_time":3600000000000},"min_asset":{"amount":"9000000","ticker":"USDC"},"min_sell_asset":{"amount":"5000","ticker":"ATOM"}}"#);
        assert_err(r, "'ATOM' pretending to be");
    }

    fn assert_load_ok(exp: PositionSpecDTO, json: &[u8]) {
        assert_eq!(Ok(exp), from_json::<PositionSpecDTO>(json));
    }

    fn assert_err(r: Result<PositionSpecDTO, StdError>, msg: &str) {
        assert!(matches!(
            r,
            Err(StdError::ParseErr {
                target_type,
                msg: real_msg
            }) if target_type.contains("PositionSpec") && real_msg.contains(msg)
        ));
    }
}
