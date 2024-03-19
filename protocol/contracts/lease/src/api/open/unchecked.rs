use serde::Deserialize;

use finance::{duration::Duration, liability::Liability, percent::Percent};
use sdk::cosmwasm_std::Addr;

use crate::{api::LpnCoinDTO, error::ContractError};

use super::{
    InterestPaymentSpec as ValidatedInterestPaymentSpec, LoanForm as LastVersionLoanForm,
    PositionSpecDTO as ValidatedPositionSpec,
};

/// Migrates v0.4.2 LoanForm instance to the next v0.5.0 format
/// TODO clean-up the v0.4.2 support once all leases have gone through this migration
#[derive(Deserialize)]
pub(super) struct LoanForm {
    lpp: Addr,
    profit: Addr,
    annual_margin_interest: Percent,
    // v0.5.0 fields follow
    #[serde(default)]
    due_period: Duration,
    // v0.4.2 fields follow
    interest_payment: Option<InterestPaymentSpec>,
}

impl From<LoanForm> for LastVersionLoanForm {
    fn from(value: LoanForm) -> Self {
        if value.interest_payment.is_some() {
            assert_eq!(value.due_period, Duration::default());

            // v0.4.2 detected
            let due_period = value
                .interest_payment
                .expect("Due period to be present in v0.4.2 data")
                .due_period;
            assert_ne!(due_period, Duration::default());
            Self {
                lpp: value.lpp,
                profit: value.profit,
                annual_margin_interest: value.annual_margin_interest,
                due_period,
            }
        } else {
            assert!(value.interest_payment.is_none());

            // v0.5.0 detected
            Self {
                lpp: value.lpp,
                profit: value.profit,
                annual_margin_interest: value.annual_margin_interest,
                due_period: value.due_period,
            }
        }
    }
}

/// Brings invariant checking as a step in deserializing a InterestPaymentSpec
#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub(super) struct InterestPaymentSpec {
    due_period: Duration,
    grace_period: Duration,
}

impl TryFrom<InterestPaymentSpec> for ValidatedInterestPaymentSpec {
    type Error = ContractError;

    fn try_from(dto: InterestPaymentSpec) -> Result<Self, Self::Error> {
        let res = Self {
            due_period: dto.due_period,
            grace_period: dto.grace_period,
        };
        res.invariant_held().map(|_| res)
    }
}

/// Brings invariant checking as a step in deserializing a PositionSpecDTO
#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub(super) struct PositionSpecDTO {
    liability: Liability,
    min_asset: LpnCoinDTO,
    min_transaction: LpnCoinDTO,
}

impl TryFrom<PositionSpecDTO> for ValidatedPositionSpec {
    type Error = ContractError;

    fn try_from(value: PositionSpecDTO) -> Result<Self, Self::Error> {
        let res = Self {
            liability: value.liability,
            min_asset: value.min_asset,
            min_transaction: value.min_transaction,
        };
        res.invariant_held().map(|_| res)
    }
}

#[cfg(test)]
mod test {
    use finance::{duration::Duration, percent::Percent};
    use sdk::cosmwasm_std::{from_json, to_json_vec, Addr};

    use crate::api::open::LoanForm;

    const LPP_ADDR: &str = "nolus1qg5ega6dykkxc307y25pecuufrjkxkaggkkxh7nad0vhyhtuhw3sqaa3c5";
    const PROFIT_ADDR: &str = "nolus1mf6ptkssddfmxvhdx0ech0k03ktp6kf9yk59renau2gvht3nq2gqkxgywu";
    const DUE_PERIOD: Duration = Duration::from_nanos(604800000000000);
    const MARGIN_INTEREST: Percent = Percent::from_permille(40);

    #[test]
    fn read_4_2_into_5_0() {
        let raw_4_2 = format!(
            r#"{{"lpp":"{LPP_ADDR}","profit":"{PROFIT_ADDR}","annual_margin_interest":{interest},
                "interest_payment":{{"due_period":{due_period},"grace_period":172800000000000}}}}"#,
            due_period = DUE_PERIOD.nanos(),
            interest = MARGIN_INTEREST.units(),
        );

        assert_eq!(loan_v5_0(), from_json(raw_4_2.clone()).unwrap());
        assert_eq!(
            to_json_vec(&loan_v5_0()).expect("serialization passed"),
            to_json_vec(&from_json::<LoanForm>(raw_4_2).expect("deserialization passed"))
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

    fn loan_v5_0() -> LoanForm {
        LoanForm {
            lpp: Addr::unchecked(LPP_ADDR),
            profit: Addr::unchecked(PROFIT_ADDR),
            annual_margin_interest: MARGIN_INTEREST,
            due_period: DUE_PERIOD,
        }
    }
}
