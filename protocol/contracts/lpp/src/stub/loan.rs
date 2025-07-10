use std::{marker::PhantomData, result::Result as StdResult};

use currency::{CurrencyDef, Group, MemberOf};
use finance::{coin::Coin, percent::Percent100};
use platform::batch::Batch;
use sdk::cosmwasm_std::Timestamp;
use thiserror::Error;

use crate::{
    loan::{Loan, RepayShares},
    msg::ExecuteMsg,
};

use super::{LppBatch, LppRef};

pub trait LppLoan<Lpn>
where
    Self: Into<LppRef<Lpn>> + TryInto<LppBatch<LppRef<Lpn>>, Error = Error>,
{
    fn principal_due(&self) -> Coin<Lpn>;
    fn interest_due(&self, by: &Timestamp) -> Coin<Lpn>;
    /// Repay the due interest and principal by the specified time
    ///
    /// First, the provided 'repayment' is used to repay the due interest,
    /// and then, if there is any remaining amount, to repay the principal.
    /// Amount 0 is acceptable although does not change the loan.
    fn repay(&mut self, by: &Timestamp, repayment: Coin<Lpn>) -> RepayShares<Lpn>;
    fn annual_interest_rate(&self) -> Percent100;
}

#[derive(Error, Debug, PartialEq)]
pub enum Error {
    #[error("[Lpp][Loan] {0}")]
    Platform(platform::error::Error),
}

pub trait WithLppLoan<Lpn> {
    type Output;
    type Error;

    fn exec<Loan>(self, loan: Loan) -> StdResult<Self::Output, Self::Error>
    where
        Loan: LppLoan<Lpn>;
}

pub(super) struct LppLoanImpl<Lpn> {
    lpp_ref: LppRef<Lpn>,
    lpn: PhantomData<Lpn>,
    loan: Loan<Lpn>,
    repayment: Coin<Lpn>,
}

impl<Lpn> LppLoanImpl<Lpn> {
    pub(super) fn new(lpp_ref: LppRef<Lpn>, loan: Loan<Lpn>) -> Self {
        Self {
            lpp_ref,
            lpn: PhantomData,
            loan,
            repayment: Default::default(),
        }
    }
}
impl<Lpn> LppLoan<Lpn> for LppLoanImpl<Lpn>
where
    Lpn: CurrencyDef,
{
    fn principal_due(&self) -> Coin<Lpn> {
        self.loan.principal_due
    }

    fn interest_due(&self, by: &Timestamp) -> Coin<Lpn> {
        self.loan.interest_due(by)
    }

    fn repay(&mut self, by: &Timestamp, repayment: Coin<Lpn>) -> RepayShares<Lpn> {
        self.repayment += repayment;
        self.loan.repay(by, repayment)
    }

    fn annual_interest_rate(&self) -> Percent100 {
        self.loan.annual_interest_rate
    }
}

impl<Lpn> From<LppLoanImpl<Lpn>> for LppRef<Lpn>
where
    Lpn: CurrencyDef,
{
    fn from(stub: LppLoanImpl<Lpn>) -> Self {
        stub.lpp_ref
    }
}

impl<Lpn> TryFrom<LppLoanImpl<Lpn>> for LppBatch<LppRef<Lpn>>
where
    Lpn: CurrencyDef,
{
    type Error = Error;

    fn try_from(stub: LppLoanImpl<Lpn>) -> StdResult<Self, Self::Error> {
        let mut batch = Batch::default();
        if !stub.repayment.is_zero() {
            batch
                .schedule_execute_wasm_no_reply(
                    stub.lpp_ref.addr().clone(),
                    &ExecuteMsg::<Lpn::Group>::RepayLoan(),
                    Some(stub.repayment),
                )
                .map_err(Self::Error::Platform)?;
        }
        Ok(Self {
            lpp_ref: stub.lpp_ref,
            batch,
        })
    }
}

#[cfg(test)]
mod test {
    use currencies::{Lpn, Lpns};
    use finance::{coin::Coin, duration::Duration, percent::Percent100, zero::Zero};
    use platform::batch::Batch;
    use sdk::cosmwasm_std::Timestamp;

    use crate::{
        loan::Loan,
        msg::ExecuteMsg,
        stub::{LppBatch, LppRef, loan::LppLoan},
    };

    use super::LppLoanImpl;

    #[test]
    fn try_from_no_payments() {
        let lpp_ref = LppRef::<Lpn>::unchecked("lpp_address");
        let start = Timestamp::from_seconds(10);
        let mut loan = LppLoanImpl::new(
            lpp_ref.clone(),
            Loan {
                principal_due: Coin::<Lpn>::new(100),
                annual_interest_rate: Percent100::from_percent(12),
                interest_paid: start,
            },
        );
        loan.repay(&(start + Duration::YEAR), Coin::ZERO);
        let batch: LppBatch<LppRef<Lpn>> = loan.try_into().unwrap();

        assert_eq!(lpp_ref, batch.lpp_ref);
        assert_eq!(Batch::default(), batch.batch);
    }

    #[test]
    fn try_from_a_few_payments() {
        let lpp_ref = LppRef::<Lpn>::unchecked("lpp_address");
        let start = Timestamp::from_seconds(0);
        let mut loan = LppLoanImpl::new(
            lpp_ref.clone(),
            Loan {
                principal_due: Coin::<Lpn>::new(100),
                annual_interest_rate: Percent100::from_percent(12),
                interest_paid: start,
            },
        );
        let payment1 = 8.into();
        let payment2 = 4.into();
        loan.repay(&(start + Duration::YEAR), payment1);
        loan.repay(&(start + Duration::YEAR), payment2);
        let batch: LppBatch<LppRef<Lpn>> = loan.try_into().unwrap();

        assert_eq!(lpp_ref, batch.lpp_ref);
        {
            let mut exp = Batch::default();
            exp.schedule_execute_wasm_no_reply(
                lpp_ref.addr().clone(),
                &ExecuteMsg::<Lpns>::RepayLoan(),
                Some(payment1 + payment2),
            )
            .unwrap();
            assert_eq!(exp, batch.batch);
        }
    }
}
