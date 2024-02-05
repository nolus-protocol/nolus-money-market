use cosmwasm_std::Timestamp;
use finance::{
    duration::Duration,
    interest::InterestPeriod,
    percent::{Percent, Units},
};
use lpp::stub::LppRef;
use profit::stub::ProfitRef;
use serde::Deserialize;

use crate::api::open::InterestPaymentSpec;

use super::LoanDTO as LastVersionLoanDTO;

/// Migrates v0.4.2 LoanDTO instance to the next v0.5.0 format
#[derive(Deserialize)]
#[serde(untagged)] // TODO clean-up the v0.4.2 support once all leases have gone through this migration
pub(super) enum LoanDTO {
    V0_5_0 {
        // this is the first variant since the untagged enum representation tries to deserialize to variants as per their order in the definition
        lpp: LppRef,
        profit: ProfitRef,
        due_period: Duration,
        margin_interest: Percent,
        margin_paid_by: Timestamp, // only this one should vary!
    },
    V0_4_2 {
        lpp: LppRef,
        interest_payment_spec: InterestPaymentSpec,
        current_period: InterestPeriod<Units, Percent>,
        profit: ProfitRef,
    },
}

impl From<LoanDTO> for LastVersionLoanDTO {
    fn from(value: LoanDTO) -> Self {
        match value {
            LoanDTO::V0_4_2 {
                lpp,
                interest_payment_spec,
                current_period,
                profit,
            } => LastVersionLoanDTO {
                lpp,
                profit,
                due_period: interest_payment_spec.due_period(),
                margin_interest: current_period.interest_rate(),
                margin_paid_by: current_period.start(),
            },

            LoanDTO::V0_5_0 {
                lpp,
                profit,
                due_period,
                margin_interest,
                margin_paid_by,
            } => LastVersionLoanDTO {
                lpp,
                profit,
                due_period,
                margin_interest,
                margin_paid_by,
            },
        }
    }
}

#[cfg(test)]
mod test_two_versions {

    use cosmwasm_std::Timestamp;
    use finance::{duration::Duration, percent::Percent};
    use lpp::stub::LppRef;
    use profit::stub::ProfitRef;
    use sdk::cosmwasm_std::{from_json, to_json_vec};

    use crate::loan::{tests::Lpn, LoanDTO};

    #[test]
    fn read_4_2_into_5_0() {
        const RAW_4_2: &str = r#"{"lpp":{"addr":"nolus1qg5ega6dykkxc307y25pecuufrjkxkaggkkxh7nad0vhyhtuhw3sqaa3c5","currency":"USDC"},
                                "profit":{"addr":"nolus1mf6ptkssddfmxvhdx0ech0k03ktp6kf9yk59renau2gvht3nq2gqkxgywu"},
                                "interest_payment_spec":{"due_period":604800000000000,"grace_period":172800000000000},
                                "current_period":{"period":{"start":"1706820166180052443","length":0},"interest":40}}"#;

        let loan_v5_0 = LoanDTO {
            lpp: LppRef::unchecked::<_, Lpn>(
                "nolus1qg5ega6dykkxc307y25pecuufrjkxkaggkkxh7nad0vhyhtuhw3sqaa3c5",
            ),
            profit: ProfitRef::unchecked::<_>(
                "nolus1mf6ptkssddfmxvhdx0ech0k03ktp6kf9yk59renau2gvht3nq2gqkxgywu",
            ),
            due_period: Duration::from_nanos(604800000000000),
            margin_interest: Percent::from_permille(40),
            margin_paid_by: Timestamp::from_nanos(1706820166180052443),
        };

        assert_eq!(loan_v5_0, from_json(RAW_4_2).unwrap());
        {
            assert_eq!(
                loan_v5_0,
                from_json(to_json_vec(&loan_v5_0).expect("serialization passed"))
                    .expect("deserialization passed")
            );
        }
    }
}
