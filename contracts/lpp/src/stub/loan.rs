use std::{marker::PhantomData, result::Result as StdResult};

use cosmwasm_std::Timestamp;
use finance::{coin::Coin, currency::Currency, percent::Percent};
use platform::batch::Batch;
use serde::{de::DeserializeOwned, Serialize};

use crate::{
    error::{ContractError, Result},
    loan::Loan,
    msg::ExecuteMsg,
};

use super::{LppBatch, LppRef};

pub trait LppLoan<Lpn>
where
    Self: Into<LppBatch<LppRef>>,
    Lpn: Currency,
{
    fn principal_due(&self) -> Coin<Lpn>;
    fn interest_due(&self, by: Timestamp) -> Coin<Lpn>;
    fn repay(&mut self, by: Timestamp, repayment: Coin<Lpn>) -> Result<()>;
    fn annual_interest_rate(&self) -> Percent;
}

pub trait WithLppLoan {
    type Output;
    type Error;

    fn exec<Lpn, Loan>(self, loan: Loan) -> StdResult<Self::Output, Self::Error>
    where
        Lpn: Currency + Serialize,
        Loan: LppLoan<Lpn>;
}

pub(super) struct LppLoanImpl<Lpn>
where
    Lpn: Currency,
{
    lpp_ref: LppRef,
    currency: PhantomData<Lpn>,
    loan: Loan<Lpn>,
    batch: Batch,
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
            batch: Batch::default(),
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

    fn repay(&mut self, by: Timestamp, repayment: Coin<Lpn>) -> Result<()> {
        self.loan.repay(by, repayment)?;
        self.batch
            .schedule_execute_wasm_no_reply(
                &self.lpp_ref.addr,
                ExecuteMsg::RepayLoan(),
                Some(repayment),
            )
            .map_err(ContractError::from)
    }

    fn annual_interest_rate(&self) -> Percent {
        self.loan.annual_interest_rate
    }
}

impl<Lpn> From<LppLoanImpl<Lpn>> for LppBatch<LppRef>
where
    Lpn: Currency,
{
    fn from(stub: LppLoanImpl<Lpn>) -> Self {
        Self {
            lpp_ref: stub.lpp_ref,
            batch: stub.batch,
        }
    }
}
