use std::{marker::PhantomData, result::Result as StdResult};

use cosmwasm_std::Timestamp;
use currency::Currency;
use finance::{coin::Coin, percent::Percent};
use platform::batch::Batch;
use serde::de::DeserializeOwned;

use crate::{
    error::ContractError,
    loan::{Loan, RepayShares},
    msg::ExecuteMsg,
};

use super::{LppBatch, LppRef};

pub trait LppLoan<Lpn>
where
    Self: TryInto<LppBatch<LppRef>, Error = ContractError>,
    Lpn: Currency,
{
    fn principal_due(&self) -> Coin<Lpn>;
    fn interest_due(&self, by: Timestamp) -> Coin<Lpn>;
    /// Repay the due interest and principal by the specified time
    ///
    /// First, the provided 'repayment' is used to repay the due interest,
    /// and then, if there is any remaining amount, to repay the principal.
    /// Amount 0 is acceptable although does not change the loan.
    fn repay(&mut self, by: Timestamp, repayment: Coin<Lpn>) -> RepayShares<Lpn>;
    fn annual_interest_rate(&self) -> Percent;
}

pub trait WithLppLoan {
    type Output;
    type Error;

    fn exec<Lpn, Loan>(self, loan: Loan) -> StdResult<Self::Output, Self::Error>
    where
        Lpn: Currency,
        Loan: LppLoan<Lpn>;
}

pub(super) struct LppLoanImpl<Lpn>
where
    Lpn: Currency,
{
    lpp_ref: LppRef,
    currency: PhantomData<Lpn>,
    loan: Loan<Lpn>,
    repayment: Coin<Lpn>,
}

impl<Lpn> LppLoanImpl<Lpn>
where
    Lpn: Currency + DeserializeOwned,
{
    pub(super) fn new(lpp_ref: LppRef, loan: Loan<Lpn>) -> Self {
        Self {
            lpp_ref,
            currency: PhantomData,
            loan,
            repayment: Default::default(),
        }
    }
}
impl<Lpn> LppLoan<Lpn> for LppLoanImpl<Lpn>
where
    Lpn: Currency,
{
    fn principal_due(&self) -> Coin<Lpn> {
        self.loan.principal_due
    }

    fn interest_due(&self, by: Timestamp) -> Coin<Lpn> {
        self.loan.interest_due(by)
    }

    fn repay(&mut self, by: Timestamp, repayment: Coin<Lpn>) -> RepayShares<Lpn> {
        self.repayment += repayment;
        self.loan.repay(by, repayment)
    }

    fn annual_interest_rate(&self) -> Percent {
        self.loan.annual_interest_rate
    }
}

impl<Lpn> TryFrom<LppLoanImpl<Lpn>> for LppBatch<LppRef>
where
    Lpn: Currency,
{
    type Error = ContractError;

    fn try_from(stub: LppLoanImpl<Lpn>) -> StdResult<Self, Self::Error> {
        let mut batch = Batch::default();
        if !stub.repayment.is_zero() {
            batch.schedule_execute_wasm_no_reply(
                &stub.lpp_ref.addr,
                ExecuteMsg::RepayLoan(),
                Some(stub.repayment),
            )?;
        }
        Ok(Self {
            lpp_ref: stub.lpp_ref,
            batch,
        })
    }
}

#[cfg(test)]
mod test {
    use cosmwasm_std::Timestamp;
    use currency::test::Usdc;
    use finance::{coin::Coin, duration::Duration, percent::Percent, zero::Zero};
    use platform::batch::Batch;

    use crate::{
        loan::Loan,
        msg::ExecuteMsg,
        stub::{loan::LppLoan, LppBatch, LppRef},
    };

    use super::LppLoanImpl;

    #[test]
    fn try_from_no_payments() {
        let lpp_ref = LppRef::unchecked::<_, Usdc>("lpp_address");
        let start = Timestamp::from_seconds(10);
        let mut loan = LppLoanImpl::new(
            lpp_ref.clone(),
            Loan {
                principal_due: Coin::<Usdc>::new(100),
                annual_interest_rate: Percent::from_percent(12),
                interest_paid: start,
            },
        );
        loan.repay(start + Duration::YEAR, Coin::ZERO);
        let batch: LppBatch<LppRef> = loan.try_into().unwrap();
        assert_eq!(lpp_ref, batch.lpp_ref);
        assert_eq!(Batch::default(), batch.batch);
    }

    #[test]
    fn try_from_a_few_payments() {
        let lpp_ref = LppRef::unchecked::<_, Usdc>("lpp_address");
        let start = Timestamp::from_seconds(0);
        let mut loan = LppLoanImpl::new(
            lpp_ref.clone(),
            Loan {
                principal_due: Coin::<Usdc>::new(100),
                annual_interest_rate: Percent::from_percent(12),
                interest_paid: start,
            },
        );
        let payment1 = 8.into();
        let payment2 = 4.into();
        loan.repay(start + Duration::YEAR, payment1);
        loan.repay(start + Duration::YEAR, payment2);
        let batch: LppBatch<LppRef> = loan.try_into().unwrap();
        assert_eq!(lpp_ref, batch.lpp_ref);
        {
            let mut exp = Batch::default();
            exp.schedule_execute_wasm_no_reply(
                lpp_ref.addr(),
                ExecuteMsg::RepayLoan(),
                Some(payment1 + payment2),
            )
            .unwrap();
            assert_eq!(exp, batch.batch);
        }
    }
}
