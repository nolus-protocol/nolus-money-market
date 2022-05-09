use cosmwasm_std::Uint128;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{
    amount::Amount,
    error::{ContractError, ContractResult},
    percent::HUNDRED,
};

#[derive(Serialize, Deserialize, Copy, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct Liability {
    /// The initial percentage of the amount due versus the locked collateral
    /// init_percent > 0
    init_percent: u8,
    /// The healty percentage of the amount due versus the locked collateral
    /// healthy_percent >= init_percent
    healthy_percent: u8,
    /// The maximum percentage of the amount due versus the locked collateral
    /// max_percent > healthy_percent
    max_percent: u8,
    /// At what time cadence to recalculate the liability
    /// recalc_secs >= 3600
    recalc_secs: u32,
}

const SECS_IN_HOUR: u32 = 60 * 60; // TODO move to a duration lib?

impl Liability {
    pub fn new(
        init_percent: u8,
        delta_to_healthy_percent: u8,
        delta_to_max_percent: u8,
        recalc_hours: u16,
    ) -> Self {
        assert!(init_percent > 0);
        assert!(delta_to_max_percent > 0);
        assert_ne!(
            init_percent.checked_add(delta_to_healthy_percent),
            None,
            "healthy percent overflow"
        );
        let healthy_percent = init_percent + delta_to_healthy_percent;

        assert_ne!(
            healthy_percent.checked_add(delta_to_max_percent),
            None,
            "max percent overflow"
        );
        let max_percent = healthy_percent + delta_to_max_percent;
        assert!(recalc_hours > 0);

        let obj = Self {
            init_percent,
            healthy_percent,
            max_percent,
            recalc_secs: u32::from(recalc_hours) * SECS_IN_HOUR,
        };
        debug_assert!(obj.invariant_held().is_ok());
        obj
    }

    pub fn invariant_held(&self) -> ContractResult<()> {
        // TODO restrict further the accepted percents to 100 since there is no much sense of having no borrow
        if self.init_percent > 0
            && self.healthy_percent >= self.init_percent
            && self.max_percent > self.healthy_percent
            && self.recalc_secs >= SECS_IN_HOUR
        {
            Result::Ok(())
        } else {
            Result::Err(ContractError::broken_invariant_err::<Liability>())
        }
    }

    pub fn init_borrow_amount(&self, downpayment: Amount) -> Amount {
        let init = self.init_percent.into();
        debug_assert!(init < HUNDRED);
        let borrowed = Uint128::from(downpayment.percent(init))
            .multiply_ratio(HUNDRED.u8(), (HUNDRED - init).u8());
        borrowed.into()
    }
}

#[cfg(test)]
mod test {
    use cosmwasm_std::from_slice;

    use crate::{amount::Amount, error::ContractError, percent::Percent};

    use super::{Liability, SECS_IN_HOUR};

    #[test]
    fn new_valid() {
        let obj = Liability::new(10, 0, 5, 20);
        assert_eq!(
            Liability {
                init_percent: 10,
                healthy_percent: 10,
                max_percent: 15,
                recalc_secs: 20 * SECS_IN_HOUR,
            },
            obj,
        );
    }

    #[test]
    fn new_edge_case() {
        let obj = Liability::new(1, 0, 1, 1);
        assert_eq!(
            Liability {
                init_percent: 1,
                healthy_percent: 1,
                max_percent: 2,
                recalc_secs: SECS_IN_HOUR,
            },
            obj,
        );
    }

    #[test]
    #[should_panic]
    fn new_invalid_init_percent() {
        Liability::new(0, 0, 1, 1);
    }

    #[test]
    #[should_panic]
    fn new_overflow_healthy_percent() {
        Liability::new(45, u8::MAX - 45 + 1, 1, 1);
    }

    #[test]
    #[should_panic]
    fn new_invalid_delta_max_percent() {
        Liability::new(10, 5, 0, 1);
    }

    #[test]
    #[should_panic]
    fn new_overflow_max_percent() {
        Liability::new(10, 5, u8::MAX - 10 - 5 + 1, 1);
    }

    #[test]
    #[should_panic]
    fn new_invalid_recalc_hours() {
        Liability::new(10, 5, 10, 0);
    }

    #[test]
    fn deserialize_invalid_state() {
        let deserialized: Liability = from_slice(
            br#"{"init_percent":40,"healthy_percent":30,"max_percent":20,"recalc_secs":36000}"#,
        )
        .unwrap();
        assert_eq!(
            ContractError::broken_invariant_err::<Liability>(),
            deserialized.invariant_held().unwrap_err()
        );
    }

    fn test_init_borrow_amount<D, P, B>(d: D, p: P, exp: B)
    where
        D: Into<Amount>,
        P: Into<Percent>,
        B: Into<Amount>,
    {
        let downpayment = d.into();
        let percent = p.into();
        let calculated = Liability {
            init_percent: percent.u8(),
            healthy_percent: 99,
            max_percent: 100,
            recalc_secs: 20000,
        }
        .init_borrow_amount(downpayment);
        assert_eq!(exp.into(), calculated);
        assert_eq!(calculated, (downpayment + calculated).percent(percent));
    }

    #[test]
    fn init_borrow() {
        test_init_borrow_amount(1000, 10, 111);
        test_init_borrow_amount(1, 10, 0);
        test_init_borrow_amount(1000, 99, 990 * 100);
    }
}
