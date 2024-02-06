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
/// TODO clean-up the v0.4.2 support once all leases have gone through this migration
#[derive(Deserialize)]
pub(super) struct LoanDTO {
    lpp: LppRef,
    profit: ProfitRef,
    // v0.5.0 fields follow
    #[serde(default)]
    due_period: Duration,
    #[serde(default)]
    margin_interest: Percent,
    #[serde(default)]
    margin_paid_by: Timestamp,
    // v0.4.2 fields follow
    interest_payment_spec: Option<InterestPaymentSpec>,
    current_period: Option<InterestPeriod<Units, Percent>>,
}

impl From<LoanDTO> for LastVersionLoanDTO {
    fn from(value: LoanDTO) -> Self {
        if value.interest_payment_spec.is_some() {
            assert!(value.current_period.is_some());
            assert_eq!(value.margin_paid_by, Timestamp::default());

            // v0.4.2 detected
            let current_period = value
                .current_period
                .expect("Current period to be present in v0.4.2 data");
            Self {
                lpp: value.lpp,
                profit: value.profit,
                due_period: value
                    .interest_payment_spec
                    .expect("Interest payment spec to be present in v0.4.2 data")
                    .due_period(),
                margin_interest: current_period.interest_rate(),
                margin_paid_by: current_period.start(),
            }
        } else {
            assert_ne!(value.margin_paid_by, Timestamp::default());

            // v0.5.0 detected
            LastVersionLoanDTO {
                lpp: value.lpp,
                profit: value.profit,
                due_period: value.due_period,
                margin_interest: value.margin_interest,
                margin_paid_by: value.margin_paid_by,
            }
        }
    }
}

#[cfg(test)]
mod test_two_versions {

    use cosmwasm_std::Timestamp;
    use currency::Currency;
    use finance::{duration::Duration, percent::Percent};
    use lpp::stub::LppRef;
    use profit::stub::ProfitRef;
    use sdk::cosmwasm_std::{from_json, to_json_vec};

    use crate::loan::{tests::Lpn, LoanDTO};

    fn loan_v5_0() -> LoanDTO {
        LoanDTO {
            lpp: LppRef::unchecked::<_, Lpn>(
                "nolus1qg5ega6dykkxc307y25pecuufrjkxkaggkkxh7nad0vhyhtuhw3sqaa3c5",
            ),
            profit: ProfitRef::unchecked::<_>(
                "nolus1mf6ptkssddfmxvhdx0ech0k03ktp6kf9yk59renau2gvht3nq2gqkxgywu",
            ),
            due_period: Duration::from_nanos(604800000000000),
            margin_interest: Percent::from_permille(40),
            margin_paid_by: Timestamp::from_nanos(1706820166180052443),
        }
    }

    #[test]
    fn read_4_2_into_5_0() {
        let raw_4_2 = format!(
            r#"{{"lpp":{{"addr":"nolus1qg5ega6dykkxc307y25pecuufrjkxkaggkkxh7nad0vhyhtuhw3sqaa3c5","currency":{:?}}},
        "profit":{{"addr":"nolus1mf6ptkssddfmxvhdx0ech0k03ktp6kf9yk59renau2gvht3nq2gqkxgywu"}},
        "interest_payment_spec":{{"due_period":604800000000000,"grace_period":172800000000000}},
        "current_period":{{"period":{{"start":"1706820166180052443","length":0}},"interest":40}}}}"#,
            Lpn::TICKER
        );

        assert_eq!(loan_v5_0(), from_json(raw_4_2.clone()).unwrap());
        assert_eq!(
            to_json_vec(&loan_v5_0()).expect("serialization passed"),
            to_json_vec(&from_json::<LoanDTO>(raw_4_2).expect("deserialization passed"))
                .expect("serialization passed")
        );
    }

    #[test]
    fn read_5_0() {
        assert_eq!(
            loan_v5_0(),
            from_json(to_json_vec(&loan_v5_0()).expect("serialization passed"))
                .expect("deserialization passed")
        );
    }
}
