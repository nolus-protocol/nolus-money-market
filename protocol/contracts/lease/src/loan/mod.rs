use serde::{Deserialize, Serialize};

use finance::{
    coin::Coin, duration::Duration, interest, percent::Percent100, period::Period, zero::Zero,
};
use lpp::stub::{LppBatch, LppRef as LppGenericRef, loan::LppLoan as LppLoanTrait};
use platform::{bank::FixedAddressSender, batch::Batch};
use profit::stub::ProfitRef;
use sdk::cosmwasm_std::Timestamp;

use crate::{
    error::{ContractError, ContractResult},
    finance::{LpnCoin, LpnCurrency},
};

pub(crate) use self::repay::Receipt as RepayReceipt;
pub use self::state::{Overdue, State};

mod repay;
mod state;

type LppRef = LppGenericRef<LpnCurrency>;

#[derive(Serialize, Deserialize, Clone)]
#[cfg_attr(feature = "contract_testing", derive(Debug, PartialEq))]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub(crate) struct LoanDTO {
    lpp: LppRef,
    profit: ProfitRef,
    due_period: Duration,
    margin_interest: Percent100,
    margin_paid_by: Timestamp, // only this one should vary!
}

impl LoanDTO {
    pub(crate) fn annual_margin_interest(&self) -> Percent100 {
        self.margin_interest
    }

    pub(crate) fn lpp(&self) -> &LppRef {
        &self.lpp
    }

    pub(crate) fn profit(&self) -> &ProfitRef {
        &self.profit
    }
}

#[cfg_attr(test, derive(Debug))]
pub(crate) struct Loan<LppLoan> {
    lpp_loan: LppLoan,
    due_period: Duration,
    margin_interest: Percent100,
    margin_paid_by: Timestamp, // only this one should vary!
}

impl<LppLoan> Loan<LppLoan>
where
    LppLoan: LppLoanTrait<LpnCurrency>,
    LppLoan::Error: Into<ContractError>,
{
    pub(super) fn into_dto(self, profit: ProfitRef) -> LoanDTO {
        LoanDTO {
            lpp: self.lpp_loan.into(),
            profit,
            due_period: self.due_period,
            margin_interest: self.margin_interest,
            margin_paid_by: self.margin_paid_by,
        }
    }

    pub(super) fn try_into_dto(self, profit: ProfitRef) -> ContractResult<(LoanDTO, Batch)> {
        Self::try_loan_into(self.lpp_loan).map(|lpp_batch: LppBatch<LppRef>| {
            (
                LoanDTO {
                    lpp: lpp_batch.lpp_ref,
                    profit,
                    due_period: self.due_period,
                    margin_interest: self.margin_interest,
                    margin_paid_by: self.margin_paid_by,
                },
                lpp_batch.batch,
            )
        })
    }

    pub(super) fn try_into_messages(self) -> ContractResult<Batch> {
        Self::try_loan_into(self.lpp_loan).map(|lpp_batch: LppBatch<LppRef>| lpp_batch.batch)
    }

    fn try_loan_into(loan: LppLoan) -> ContractResult<LppBatch<LppRef>> {
        loan.try_into().map_err(Into::<ContractError>::into)
    }
}

impl<LppLoan> Loan<LppLoan>
where
    LppLoan: LppLoanTrait<LpnCurrency>,
{
    pub(super) fn new(
        lpp_loan: LppLoan,
        start: Timestamp,
        annual_margin_interest: Percent100,
        due_period: Duration,
    ) -> Self {
        Self {
            lpp_loan,
            due_period,
            margin_interest: annual_margin_interest,
            margin_paid_by: start,
        }
    }

    pub(super) fn from_dto(dto: LoanDTO, lpp_loan: LppLoan) -> Self {
        Self {
            lpp_loan,
            due_period: dto.due_period,
            margin_interest: dto.margin_interest,
            margin_paid_by: dto.margin_paid_by,
        }
    }

    /// Repay the loan interests and principal by the given timestamp.
    ///
    /// The time intervals are always open-ended!
    pub(crate) fn repay<Profit>(
        &mut self,
        payment: LpnCoin,
        by: &Timestamp,
        profit: &mut Profit,
    ) -> ContractResult<RepayReceipt>
    where
        Profit: FixedAddressSender,
    {
        self.debug_check_start_due_before(by, "before the 'repay-by' time");

        let state = self.state(by)?;
        let overdue_interest_payment = state.overdue.interest().min(payment);
        let overdue_margin_payment = state
            .overdue
            .margin()
            .min(payment - overdue_interest_payment);
        let due_interest_payment = state
            .due_interest
            .min(payment - overdue_interest_payment - overdue_margin_payment);
        let due_margin_payment = state.due_margin_interest.min(
            payment - overdue_interest_payment - overdue_margin_payment - due_interest_payment,
        );

        let interest_paid = overdue_interest_payment + due_interest_payment;
        let margin_paid = overdue_margin_payment + due_margin_payment;
        let principal_paid = state
            .principal_due
            .min(payment - interest_paid - margin_paid);
        let change = payment - interest_paid - margin_paid - principal_paid;
        debug_assert_eq!(
            payment,
            interest_paid + margin_paid + principal_paid + change
        );

        self.repay_margin(state.principal_due, margin_paid, by)
            .ok_or(ContractError::overflow("Repay margin overflow"))
            .inspect(|()| profit.send(margin_paid))
            .and_then(|()| {
                self.repay_loan(interest_paid, principal_paid, by)
                    .ok_or(ContractError::overflow("Repay loan overflow"))
            })
            .map(|()| {
                RepayReceipt::new(
                    overdue_interest_payment,
                    overdue_margin_payment,
                    due_interest_payment,
                    due_margin_payment,
                    state.principal_due,
                    principal_paid,
                    change,
                )
            })
            .inspect(|receipt| debug_assert_eq!(payment, receipt.total()))
    }

    pub(crate) fn state(&self, now: &Timestamp) -> Result<State, ContractError> {
        self.debug_check_start_due_before(now, "in the past. Now is ");

        let due_period_margin = Period::from_till(self.margin_paid_by, now);

        let overdue = Overdue::new(
            &due_period_margin,
            self.due_period,
            self.margin_interest,
            &self.lpp_loan,
        )?;

        let principal_due = self.lpp_loan.principal_due();
        let due_margin_interest = interest::interest(
            self.margin_interest,
            principal_due,
            due_period_margin.length(),
        )
        .ok_or(ContractError::overflow("Due interest margin overflow"))
        .map(|margin| margin - overdue.margin())?;

        let due_interest = self
            .lpp_loan
            .interest_due(&due_period_margin.till())
            .ok_or(ContractError::overflow("Due interest overflow"))
            .map(|due| due - overdue.interest())?;

        Ok(State {
            annual_interest: self.lpp_loan.annual_interest_rate(),
            annual_interest_margin: self.margin_interest,
            principal_due,
            due_interest,
            due_margin_interest,
            overdue,
        })
    }

    #[must_use]
    fn repay_margin(
        &mut self,
        principal_due: LpnCoin,
        margin_paid: LpnCoin,
        by: &Timestamp,
    ) -> Option<()> {
        interest::pay(
            self.margin_interest,
            principal_due,
            margin_paid,
            Duration::between(&self.margin_paid_by, by),
        )
        .inspect(|(_, margin_payment_change)| debug_assert!(margin_payment_change.is_zero()))
        .map(|(margin_paid_for, _)| {
            self.margin_paid_by += margin_paid_for;
        })
    }

    #[must_use]
    fn repay_loan(
        &mut self,
        interest_paid: LpnCoin,
        principal_paid: LpnCoin,
        by: &Timestamp,
    ) -> Option<()> {
        self.lpp_loan
            .repay(by, interest_paid + principal_paid)
            .inspect(|shares| {
                debug_assert_eq!(shares.interest, interest_paid);
                debug_assert_eq!(shares.principal, principal_paid);
                debug_assert_eq!(shares.excess, Coin::ZERO);
            })
            .map(|_| ())
    }

    fn debug_check_start_due_before(&self, when: &Timestamp, when_descr: &str) {
        debug_assert!(
            &self.margin_paid_by <= when,
            "The current due period starting at {}s, should begin {} {}s",
            self.margin_paid_by,
            when_descr,
            when
        );
    }
}

#[cfg(all(feature = "internal.test.contract", test))]
mod tests {
    use serde::{Deserialize, Serialize};

    pub use currencies::Lpn;
    use finance::{duration::Duration, percent::Percent100};
    use lpp::{
        loan::RepayShares,
        msg::LoanResponse,
        stub::{
            LppBatch,
            loan::{Error as LppLoanError, LppLoan as LppLoanTrait},
        },
    };
    use platform::bank::FixedAddressSender;
    use profit::stub::ProfitRef;
    use sdk::cosmwasm_std::Timestamp;

    use crate::{finance::LpnCoin, lease::tests};

    use super::{Loan, LppRef};

    const MARGIN_INTEREST_RATE: Percent100 = Percent100::from_permille(50);
    const LOAN_INTEREST_RATE: Percent100 = Percent100::from_permille(500);
    const LEASE_START: Timestamp = Timestamp::from_nanos(100);
    const PROFIT_ADDR: &str = "profit_addr";
    const ZERO_COIN: LpnCoin = tests::lpn_coin(0);

    mod test_repay {
        use finance::{
            coin::Coin,
            duration::Duration,
            fraction::{Fraction, Unit},
            percent::Percent100,
            zero::Zero,
        };
        use lpp::msg::LoanResponse;
        use platform::{bank, batch::Batch};
        use sdk::cosmwasm_std::{Addr, Timestamp};

        use crate::{
            finance::LpnCoin,
            lease::tests,
            loan::{Loan, Overdue, State, repay::Receipt as RepayReceipt},
        };

        use super::{
            LEASE_START, LOAN_INTEREST_RATE, LppLoanLocal, MARGIN_INTEREST_RATE, PROFIT_ADDR,
            ZERO_COIN,
        };

        #[test]
        fn full_max_overdue_full_max_due_repay() {
            let principal = tests::lpn_coin(1000);
            let delta_to_fully_paid = tests::lpn_coin(30);
            let payment_at = LEASE_START + Duration::YEAR + Duration::YEAR;
            let one_year_margin = MARGIN_INTEREST_RATE.of(principal);
            let one_year_interest = LOAN_INTEREST_RATE.of(principal);
            assert!(delta_to_fully_paid < one_year_margin);
            assert!(delta_to_fully_paid < one_year_interest);

            let loan = LoanResponse {
                principal_due: principal,
                annual_interest_rate: LOAN_INTEREST_RATE,
                interest_paid: LEASE_START,
            };

            let mut loan = super::create_loan(loan);
            {
                let repay_overdue_interest = one_year_interest - delta_to_fully_paid;
                repay(
                    &mut loan,
                    repay_overdue_interest,
                    state(
                        principal,
                        one_year_margin,
                        one_year_interest,
                        Overdue::Accrued {
                            interest: one_year_interest,
                            margin: one_year_margin,
                        },
                    ),
                    receipt(
                        principal,
                        ZERO_COIN,
                        ZERO_COIN,
                        repay_overdue_interest,
                        ZERO_COIN,
                        ZERO_COIN,
                        ZERO_COIN,
                    ),
                    Duration::default(),
                    &payment_at,
                )
            }

            {
                let repay_fully_overdue_interest_and_some_margin =
                    delta_to_fully_paid + delta_to_fully_paid;
                repay(
                    &mut loan,
                    repay_fully_overdue_interest_and_some_margin,
                    state(
                        principal,
                        one_year_margin,
                        one_year_interest,
                        Overdue::Accrued {
                            interest: delta_to_fully_paid,
                            margin: one_year_margin,
                        },
                    ),
                    receipt(
                        principal,
                        ZERO_COIN,
                        repay_fully_overdue_interest_and_some_margin - delta_to_fully_paid,
                        delta_to_fully_paid,
                        ZERO_COIN,
                        ZERO_COIN,
                        ZERO_COIN,
                    ),
                    Duration::default(),
                    &payment_at,
                )
            }

            {
                let overdue_margin = one_year_margin - delta_to_fully_paid;
                let repay_fully_overdue_margin_and_some_due_interest =
                    overdue_margin + delta_to_fully_paid;
                repay(
                    &mut loan,
                    repay_fully_overdue_margin_and_some_due_interest,
                    state(
                        principal,
                        one_year_margin,
                        one_year_interest,
                        Overdue::Accrued {
                            interest: ZERO_COIN,
                            margin: overdue_margin,
                        },
                    ),
                    receipt(
                        principal,
                        ZERO_COIN,
                        overdue_margin,
                        ZERO_COIN,
                        ZERO_COIN,
                        repay_fully_overdue_margin_and_some_due_interest - overdue_margin,
                        ZERO_COIN,
                    ),
                    Duration::default(),
                    &payment_at,
                )
            }

            {
                let interest_due = one_year_interest - delta_to_fully_paid;
                let surplus = delta_to_fully_paid;
                let full_repayment = interest_due + one_year_margin + principal + surplus;
                repay(
                    &mut loan,
                    full_repayment,
                    state(
                        principal,
                        one_year_margin,
                        interest_due,
                        Overdue::Accrued {
                            interest: Coin::ZERO,
                            margin: Coin::ZERO,
                        },
                    ),
                    receipt(
                        principal,
                        principal,
                        ZERO_COIN,
                        ZERO_COIN,
                        one_year_margin,
                        interest_due,
                        surplus,
                    ),
                    Duration::YEAR,
                    &payment_at,
                )
            }
        }

        #[test]
        fn partial_max_due_margin_repay() {
            let principal = tests::lpn_coin(1000);
            let due_margin = MARGIN_INTEREST_RATE.of(principal);
            let payment = due_margin.scale_down(2u128);
            let now = LEASE_START + Duration::YEAR;

            let mut loan = super::create_loan(LoanResponse {
                principal_due: principal,
                annual_interest_rate: LOAN_INTEREST_RATE,
                interest_paid: now,
            });

            repay(
                &mut loan,
                payment,
                state(
                    principal,
                    due_margin,
                    ZERO_COIN,
                    Overdue::Accrued {
                        interest: ZERO_COIN,
                        margin: ZERO_COIN,
                    },
                ),
                receipt(
                    principal, ZERO_COIN, ZERO_COIN, ZERO_COIN, payment, ZERO_COIN, ZERO_COIN,
                ),
                Duration::YEAR
                    .into_slice_per_ratio(payment, due_margin)
                    .unwrap(),
                &now,
            );
        }

        #[test]
        fn partial_overdue_interest_repay() {
            let principal = tests::lpn_coin(1000);
            let one_year_margin = MARGIN_INTEREST_RATE.of(principal);
            let one_year_interest = LOAN_INTEREST_RATE.of(principal);
            let overdue_period = Duration::from_days(100);
            let overdue_interest = overdue_period
                .annualized_slice_of(one_year_interest)
                .unwrap();
            let overdue_margin = overdue_period.annualized_slice_of(one_year_margin).unwrap();

            let partial_overdue_interest = overdue_interest - tests::lpn_coin(10);
            let repay_at = LEASE_START + Duration::YEAR + overdue_period;

            let loan = LoanResponse {
                principal_due: principal,
                annual_interest_rate: LOAN_INTEREST_RATE,
                interest_paid: LEASE_START,
            };

            let mut loan = super::create_loan(loan);
            {
                let payment = partial_overdue_interest;
                repay(
                    &mut loan,
                    payment,
                    state(
                        principal,
                        one_year_margin,
                        one_year_interest,
                        Overdue::Accrued {
                            interest: overdue_interest,
                            margin: overdue_margin,
                        },
                    ),
                    receipt(
                        principal,
                        ZERO_COIN,
                        ZERO_COIN,
                        partial_overdue_interest,
                        ZERO_COIN,
                        ZERO_COIN,
                        ZERO_COIN,
                    ),
                    Duration::default(),
                    &repay_at,
                );
            }
        }

        #[test]
        fn multiple_periods() {
            let principal = tests::lpn_coin(1000);
            let one_year_margin = MARGIN_INTEREST_RATE.of(principal);
            let one_year_interest = LOAN_INTEREST_RATE.of(principal);
            let overdue_period_molulo_year = Duration::from_days(120);
            let repay_at = LEASE_START
                + overdue_period_molulo_year
                + Duration::YEAR
                + Duration::YEAR
                + Duration::YEAR;

            let overdue_margin_modulo_year = overdue_period_molulo_year
                .annualized_slice_of(one_year_margin)
                .unwrap();
            let overdue_interest_modulo_year = overdue_period_molulo_year
                .annualized_slice_of(one_year_interest)
                .unwrap();
            let interest_payment = overdue_interest_modulo_year - tests::lpn_coin(10);

            let loan = LoanResponse {
                principal_due: principal,
                annual_interest_rate: LOAN_INTEREST_RATE,
                interest_paid: LEASE_START,
            };

            let mut loan = super::create_loan(loan);
            {
                let payment = one_year_interest + one_year_interest + interest_payment;
                repay(
                    &mut loan,
                    payment,
                    state(
                        principal,
                        one_year_margin,
                        one_year_interest,
                        Overdue::Accrued {
                            interest: one_year_interest.checked_mul(2).unwrap()
                                + overdue_interest_modulo_year,
                            margin: one_year_margin.checked_mul(2).unwrap()
                                + overdue_margin_modulo_year,
                        },
                    ),
                    receipt(
                        principal, ZERO_COIN, ZERO_COIN, payment, ZERO_COIN, ZERO_COIN, ZERO_COIN,
                    ),
                    Duration::default(),
                    &repay_at,
                );
            }
            {
                let payment =
                    overdue_interest_modulo_year - interest_payment + overdue_margin_modulo_year;
                repay(
                    &mut loan,
                    payment,
                    state(
                        principal,
                        one_year_margin,
                        one_year_interest,
                        Overdue::Accrued {
                            interest: overdue_interest_modulo_year - interest_payment,
                            margin: one_year_margin.checked_mul(2).unwrap()
                                + overdue_margin_modulo_year,
                        },
                    ),
                    receipt(
                        principal,
                        ZERO_COIN,
                        overdue_margin_modulo_year,
                        overdue_interest_modulo_year - interest_payment,
                        ZERO_COIN,
                        ZERO_COIN,
                        ZERO_COIN,
                    ),
                    Duration::default(),
                    &repay_at,
                );
            }
            {
                let payment = one_year_margin.checked_mul(2).unwrap() + interest_payment;
                repay(
                    &mut loan,
                    payment,
                    state(
                        principal,
                        one_year_margin,
                        one_year_interest,
                        Overdue::Accrued {
                            interest: ZERO_COIN,
                            margin: one_year_margin.checked_mul(2).unwrap(),
                        },
                    ),
                    receipt(
                        principal,
                        ZERO_COIN,
                        one_year_margin.checked_mul(2).unwrap(),
                        ZERO_COIN,
                        ZERO_COIN,
                        interest_payment,
                        ZERO_COIN,
                    ),
                    Duration::default(),
                    &repay_at,
                );
            }
            {
                let change = tests::lpn_coin(3);
                let payment =
                    (one_year_interest - interest_payment) + one_year_margin + principal + change;
                repay(
                    &mut loan,
                    payment,
                    state(
                        principal,
                        one_year_margin,
                        one_year_interest - interest_payment,
                        Overdue::Accrued {
                            interest: ZERO_COIN,
                            margin: ZERO_COIN,
                        },
                    ),
                    receipt(
                        principal,
                        principal,
                        ZERO_COIN,
                        ZERO_COIN,
                        one_year_margin,
                        one_year_interest - interest_payment,
                        change,
                    ),
                    Duration::YEAR,
                    &repay_at,
                );
            }
        }

        #[test]
        fn full_max_overdue_full_due_repay() {
            let principal = tests::lpn_coin(57326);
            let due_margin_payment = tests::lpn_coin(42);
            let due_margin = MARGIN_INTEREST_RATE.of(principal);
            let due_interest = LOAN_INTEREST_RATE.of(principal);

            let loan = LoanResponse {
                principal_due: principal,
                annual_interest_rate: LOAN_INTEREST_RATE,
                interest_paid: LEASE_START,
            };

            let overdue_period =
                Duration::YEAR - Duration::HOUR - Duration::HOUR - Duration::HOUR - Duration::HOUR;
            let repay_at = LEASE_START + Duration::YEAR + overdue_period;
            let overdue_margin = overdue_period.annualized_slice_of(due_margin).unwrap();
            let overdue_interest = overdue_period.annualized_slice_of(due_interest).unwrap();
            let payment = overdue_interest + overdue_margin + due_interest + due_margin_payment;
            let due_period_paid = Duration::between(&LEASE_START, &repay_at)
                .into_slice_per_ratio(
                    overdue_margin + due_margin_payment,
                    overdue_margin + due_margin,
                )
                .unwrap()
                - overdue_period;

            let mut loan = super::create_loan(loan);
            repay(
                &mut loan,
                payment,
                state(
                    principal,
                    due_margin,
                    due_interest,
                    Overdue::Accrued {
                        interest: overdue_interest,
                        margin: overdue_margin,
                    },
                ),
                receipt(
                    principal,
                    ZERO_COIN,
                    overdue_margin,
                    overdue_interest,
                    due_margin_payment,
                    due_interest,
                    ZERO_COIN,
                ),
                due_period_paid,
                &repay_at,
            );
        }

        #[test]
        fn full_partial_due_repay() {
            let principal = tests::lpn_coin(36463892);
            let principal_paid = tests::lpn_coin(234);
            let one_year_margin = MARGIN_INTEREST_RATE.of(principal);
            let one_year_interest = LOAN_INTEREST_RATE.of(principal);
            let due_period = Duration::HOUR + Duration::HOUR + Duration::HOUR;
            let due_margin = due_period.annualized_slice_of(one_year_margin).unwrap();
            let due_interest = due_period.annualized_slice_of(one_year_interest).unwrap();
            let payment = due_margin + due_interest + principal_paid;

            let repay_at = LEASE_START + due_period;
            let mut loan = super::create_loan(LoanResponse {
                principal_due: principal,
                annual_interest_rate: LOAN_INTEREST_RATE,
                interest_paid: LEASE_START,
            });
            repay(
                &mut loan,
                payment,
                state(
                    principal,
                    due_margin,
                    due_interest,
                    Overdue::StartIn(Duration::YEAR - due_period),
                ),
                receipt(
                    principal,
                    principal_paid,
                    ZERO_COIN,
                    ZERO_COIN,
                    due_margin,
                    due_interest,
                    ZERO_COIN,
                ),
                due_period,
                &repay_at,
            );
        }

        #[test]
        fn full_zero_loan_overdue_partial_due_repay() {
            // selected to have interest > 0 and margin == 0 for the overdue period of 2 hours
            let principal = tests::lpn_coin(9818);
            let loan_interest_rate = MARGIN_INTEREST_RATE; // we aim at simulating the margin paid-by going ahead of the loan paid-by
            let margin_interest_rate = LOAN_INTEREST_RATE;
            let principal_paid = tests::lpn_coin(23);
            let due_margin = margin_interest_rate.of(principal);
            let due_interest = loan_interest_rate.of(principal);
            let overdue_period = Duration::HOUR + Duration::HOUR;
            let overdue_interest = overdue_period.annualized_slice_of(due_interest).unwrap();
            assert_eq!(ZERO_COIN, overdue_interest);
            let overdue_margin = overdue_period.annualized_slice_of(due_margin).unwrap();
            assert!(ZERO_COIN != overdue_margin);

            let loan = LoanResponse {
                principal_due: principal,
                annual_interest_rate: loan_interest_rate,
                interest_paid: LEASE_START,
            };

            let repay_at = LEASE_START + Duration::YEAR + Duration::HOUR + Duration::HOUR;
            let mut loan =
                super::create_loan_custom(margin_interest_rate, loan, LEASE_START, Duration::YEAR);
            repay(
                &mut loan,
                overdue_interest + overdue_margin,
                state_custom_percents(
                    loan_interest_rate,
                    margin_interest_rate,
                    principal,
                    due_margin,
                    due_interest,
                    Overdue::Accrued {
                        interest: overdue_interest,
                        margin: overdue_margin,
                    },
                ),
                receipt(
                    principal,
                    ZERO_COIN,
                    overdue_margin,
                    overdue_interest,
                    ZERO_COIN,
                    ZERO_COIN,
                    ZERO_COIN,
                ),
                Duration::default(),
                &repay_at,
            );
            repay(
                &mut loan,
                due_margin + due_interest + principal_paid,
                state_custom_percents(
                    loan_interest_rate,
                    margin_interest_rate,
                    principal,
                    due_margin,
                    due_interest,
                    Overdue::Accrued {
                        interest: ZERO_COIN,
                        margin: ZERO_COIN,
                    },
                ),
                receipt(
                    principal,
                    principal_paid,
                    ZERO_COIN,
                    ZERO_COIN,
                    due_margin,
                    due_interest,
                    ZERO_COIN,
                ),
                Duration::YEAR,
                &repay_at,
            );
        }

        #[test]
        fn full_principal_repay() {
            let principal = tests::lpn_coin(3646389225881);
            let principal_paid = tests::lpn_coin(234);
            let one_year_margin = MARGIN_INTEREST_RATE.of(principal);
            let one_year_interest = LOAN_INTEREST_RATE.of(principal);
            let due_period = Duration::HOUR + Duration::HOUR + Duration::HOUR;
            let due_margin = due_period.annualized_slice_of(one_year_margin).unwrap();
            let due_interest = due_period.annualized_slice_of(one_year_interest).unwrap();
            let mut loan = super::create_loan(LoanResponse {
                principal_due: principal,
                annual_interest_rate: LOAN_INTEREST_RATE,
                interest_paid: LEASE_START,
            });
            {
                let payment = due_margin + due_interest + principal_paid;
                let repay_at = LEASE_START + due_period;
                repay(
                    &mut loan,
                    payment,
                    state(
                        principal,
                        due_margin,
                        due_interest,
                        Overdue::StartIn(Duration::YEAR - due_period),
                    ),
                    receipt(
                        principal,
                        principal_paid,
                        ZERO_COIN,
                        ZERO_COIN,
                        due_margin,
                        due_interest,
                        ZERO_COIN,
                    ),
                    due_period,
                    &repay_at,
                )
            }

            {
                let principal_due = principal - principal_paid;
                let change = tests::lpn_coin(97);
                let duration_since_prev_payment = Duration::YEAR - due_period;
                let due_margin = duration_since_prev_payment
                    .annualized_slice_of(MARGIN_INTEREST_RATE.of(principal_due))
                    .unwrap();
                let due_interest = duration_since_prev_payment
                    .annualized_slice_of(LOAN_INTEREST_RATE.of(principal_due))
                    .unwrap();
                let payment = due_margin + due_interest + principal_due + change;
                let repay_at = LEASE_START + Duration::YEAR;
                repay(
                    &mut loan,
                    payment,
                    state(
                        principal_due,
                        due_margin,
                        due_interest,
                        Overdue::StartIn(due_period),
                    ),
                    receipt(
                        principal_due,
                        principal_due,
                        ZERO_COIN,
                        ZERO_COIN,
                        due_margin,
                        due_interest,
                        change,
                    ),
                    duration_since_prev_payment,
                    &repay_at,
                )
            }
        }

        #[test]
        fn repay_zero() {
            let principal = tests::lpn_coin(13);
            let total_margin = MARGIN_INTEREST_RATE.of(principal);
            let total_interest = LOAN_INTEREST_RATE.of(principal);

            let due_period = Duration::HOUR;
            let since_start = Duration::YEAR;
            let mut loan = super::create_loan_custom(
                MARGIN_INTEREST_RATE,
                LoanResponse {
                    principal_due: principal,
                    annual_interest_rate: LOAN_INTEREST_RATE,
                    interest_paid: LEASE_START,
                },
                LEASE_START,
                due_period,
            );
            let repay_at = LEASE_START + since_start;
            let principal_left = {
                let due_period_paid = Duration::default();

                let overdue_margin = (since_start - due_period)
                    .annualized_slice_of(total_margin)
                    .unwrap();
                let due_margin = total_margin - overdue_margin;
                assert_eq!(ZERO_COIN, due_margin);
                assert_eq!(ZERO_COIN, total_margin);

                let overdue_interest = (since_start - due_period)
                    .annualized_slice_of(total_interest)
                    .unwrap();
                let due_interest = total_interest - overdue_interest;
                assert_eq!(tests::lpn_coin(1), due_interest);

                let payment = tests::lpn_coin(15);
                let principal_paid =
                    payment - overdue_margin - due_margin - overdue_interest - due_interest;

                repay(
                    &mut loan,
                    payment,
                    state(
                        principal,
                        due_margin,
                        due_interest,
                        Overdue::Accrued {
                            interest: overdue_interest,
                            margin: overdue_margin,
                        },
                    ),
                    receipt(
                        principal,
                        principal_paid,
                        overdue_margin,
                        overdue_interest,
                        due_margin,
                        due_interest,
                        payment
                            - principal_paid
                            - overdue_margin
                            - overdue_interest
                            - due_margin
                            - due_interest,
                    ),
                    due_period_paid,
                    &repay_at,
                );
                principal - principal_paid
            };
            {
                let change = tests::lpn_coin(2);
                let payment = principal_left + change;
                let repay_at = LEASE_START + since_start;
                repay(
                    &mut loan,
                    payment,
                    state(
                        principal_left,
                        ZERO_COIN,
                        ZERO_COIN,
                        Overdue::Accrued {
                            interest: ZERO_COIN,
                            margin: ZERO_COIN,
                        },
                    ),
                    receipt(
                        principal_left,
                        principal_left,
                        ZERO_COIN,
                        ZERO_COIN,
                        ZERO_COIN,
                        ZERO_COIN,
                        change,
                    ),
                    Duration::default(),
                    &repay_at,
                );
            }
        }

        #[track_caller]
        fn repay(
            loan: &mut Loan<LppLoanLocal>,
            payment: LpnCoin,
            before_state: State,
            exp_receipt: RepayReceipt,
            exp_due_period_paid: Duration,
            now: &Timestamp,
        ) {
            let mut profit = super::profit_stub();

            assert_eq!(
                Ok(&before_state),
                loan.state(now).as_ref(),
                "Expected state before"
            );
            assert_eq!(Ok(exp_receipt), loan.repay(payment, now, &mut profit));
            assert_eq!(
                Ok(after_state(before_state, exp_due_period_paid, exp_receipt)),
                loan.state(now),
                "Expected state after"
            );

            assert_eq!(
                {
                    let margin_paid =
                        exp_receipt.overdue_margin_paid() + exp_receipt.due_margin_paid();
                    if margin_paid != Coin::default() {
                        bank::bank_send(Addr::unchecked(PROFIT_ADDR), margin_paid)
                    } else {
                        Batch::default()
                    }
                },
                Into::<Batch>::into(profit)
            )
        }

        fn after_state(
            before_state: State,
            exp_due_period_paid: Duration,
            exp_receipt: RepayReceipt,
        ) -> State {
            let exp_overdue = if before_state.overdue.start_in() == Duration::default() {
                let exp_interest =
                    before_state.overdue.interest() - exp_receipt.overdue_interest_paid();
                let exp_margin = before_state.overdue.margin() - exp_receipt.overdue_margin_paid();
                if exp_interest.is_zero()
                    && exp_margin.is_zero()
                    && exp_due_period_paid != Duration::default()
                {
                    Overdue::StartIn(exp_due_period_paid)
                } else {
                    Overdue::Accrued {
                        interest: exp_interest,
                        margin: exp_margin,
                    }
                }
            } else {
                Overdue::StartIn(before_state.overdue.start_in() + exp_due_period_paid)
            };
            State {
                annual_interest_margin: before_state.annual_interest_margin,
                annual_interest: before_state.annual_interest,
                principal_due: before_state.principal_due - exp_receipt.principal_paid(),
                due_margin_interest: before_state.due_margin_interest
                    - exp_receipt.due_margin_paid(),
                due_interest: before_state.due_interest - exp_receipt.due_interest_paid(),
                overdue: exp_overdue,
            }
        }

        fn state(
            principal: LpnCoin,
            due_margin_interest: LpnCoin,
            due_interest: LpnCoin,
            overdue: Overdue,
        ) -> State {
            state_custom_percents(
                LOAN_INTEREST_RATE,
                MARGIN_INTEREST_RATE,
                principal,
                due_margin_interest,
                due_interest,
                overdue,
            )
        }

        fn state_custom_percents(
            annual_interest: Percent100,
            annual_interest_margin: Percent100,
            principal: LpnCoin,
            due_margin_interest: LpnCoin,
            due_interest: LpnCoin,
            overdue: Overdue,
        ) -> State {
            State {
                annual_interest,
                annual_interest_margin,
                principal_due: principal,
                due_margin_interest,
                due_interest,
                overdue,
            }
        }

        fn receipt(
            principal: LpnCoin,
            paid_principal: LpnCoin,
            paid_overdue_margin: LpnCoin,
            paid_overdue_interest: LpnCoin,
            paid_due_margin: LpnCoin,
            paid_due_interest: LpnCoin,
            change: LpnCoin,
        ) -> RepayReceipt {
            RepayReceipt::new(
                paid_overdue_interest,
                paid_overdue_margin,
                paid_due_interest,
                paid_due_margin,
                principal,
                paid_principal,
                change,
            )
        }
    }

    #[cfg(test)]
    mod test_state {
        use finance::{duration::Duration, interest, percent::Percent100, period::Period};
        use lpp::{msg::LoanResponse, stub::loan::LppLoan};
        use sdk::cosmwasm_std::Timestamp;

        use crate::{
            lease::tests,
            loan::{Overdue, State},
        };

        use super::{LEASE_START, LppLoanLocal, MARGIN_INTEREST_RATE};

        #[track_caller]
        fn test_state(interest_paid_by: Timestamp, margin_paid_by: Timestamp, now: &Timestamp) {
            let principal_due = tests::lpn_coin(10000);
            let due_period_len = Duration::YEAR;
            let annual_interest_margin = MARGIN_INTEREST_RATE;
            let annual_interest = Percent100::from_permille(145);

            let loan_resp = LoanResponse {
                principal_due,
                annual_interest_rate: annual_interest,
                interest_paid: interest_paid_by,
            };

            let loan = super::create_loan_custom(
                MARGIN_INTEREST_RATE,
                loan_resp,
                margin_paid_by,
                due_period_len,
            );
            let due_period_margin = Period::from_till(margin_paid_by, now);
            let lpp_loan = LppLoanLocal::new(loan_resp);
            let overdue = Overdue::new(
                &due_period_margin,
                due_period_len,
                annual_interest_margin,
                &lpp_loan,
            )
            .unwrap();
            let due_period = due_period_len.min(due_period_margin.length());
            let expected_margin_due =
                interest::interest(annual_interest_margin, principal_due, due_period).unwrap();
            let interest_due = lpp_loan.interest_due(&due_period_margin.till()).unwrap();
            let expected_interest_due = interest_due - overdue.interest();

            assert_eq!(
                Ok(State {
                    annual_interest,
                    annual_interest_margin,
                    principal_due,
                    due_interest: expected_interest_due,
                    due_margin_interest: expected_margin_due,
                    overdue,
                }),
                loan.state(now),
                "Got different state than expected!",
            );
        }

        fn test_states_paid_by(since_paid: Duration) {
            let paid_by = LEASE_START + since_paid;
            test_state(LEASE_START, LEASE_START, &paid_by);
            test_state(LEASE_START, paid_by, &paid_by);
            test_state(paid_by, LEASE_START, &paid_by);
        }

        #[test]
        fn state_at_open() {
            test_states_paid_by(Duration::default())
        }

        #[test]
        fn state_in_aday() {
            test_states_paid_by(Duration::from_days(1));
        }

        #[test]
        fn state_in_half_due_period() {
            test_states_paid_by(Duration::from_days(188));
        }

        #[test]
        fn state_year() {
            test_states_paid_by(Duration::YEAR)
        }

        #[test]
        fn state_year_plus_day() {
            test_states_paid_by(Duration::YEAR + Duration::from_days(1))
        }

        #[test]
        fn state_year_minus_day() {
            test_states_paid_by(Duration::YEAR - Duration::from_days(1))
        }

        #[test]
        fn state_two_years_plus_day() {
            test_states_paid_by(Duration::YEAR + Duration::YEAR + Duration::from_days(1))
        }
    }

    // TODO migrate to using lpp::stub::unchecked_lpp_loan
    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    pub(super) struct LppLoanLocal {
        loan: LoanResponse<Lpn>,
    }

    impl LppLoanLocal {
        pub fn new(loan: LoanResponse<Lpn>) -> Self {
            Self { loan }
        }
    }

    impl LppLoanTrait<Lpn> for LppLoanLocal {
        fn principal_due(&self) -> LpnCoin {
            self.loan.principal_due
        }

        fn interest_due(&self, by: &Timestamp) -> Option<LpnCoin> {
            self.loan.interest_due(by)
        }

        fn repay(&mut self, by: &Timestamp, repayment: LpnCoin) -> Option<RepayShares<Lpn>> {
            self.loan.repay(by, repayment)
        }

        fn annual_interest_rate(&self) -> Percent100 {
            self.loan.annual_interest_rate
        }
    }

    impl From<LppLoanLocal> for LppRef {
        fn from(_: LppLoanLocal) -> Self {
            unreachable!()
        }
    }

    impl TryFrom<LppLoanLocal> for LppBatch<LppRef> {
        type Error = LppLoanError;
        fn try_from(_: LppLoanLocal) -> Result<Self, Self::Error> {
            unreachable!()
        }
    }

    fn create_loan(loan: LoanResponse<Lpn>) -> Loan<LppLoanLocal> {
        create_loan_custom(MARGIN_INTEREST_RATE, loan, LEASE_START, Duration::YEAR)
    }

    fn create_loan_custom(
        annual_margin_interest: Percent100,
        loan: LoanResponse<Lpn>,
        due_start: Timestamp,
        due_period: Duration,
    ) -> Loan<LppLoanLocal> {
        Loan::new(
            LppLoanLocal::new(loan),
            due_start,
            annual_margin_interest,
            due_period,
        )
    }

    fn profit_stub() -> impl FixedAddressSender {
        ProfitRef::unchecked(PROFIT_ADDR).into_stub()
    }
}
