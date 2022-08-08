use std::result::Result as StdResult;

use cosmwasm_std::Addr;
use finance::{
    currency::Currency,
    percent::Percent
};
use platform::batch::Batch;
use lpp::stub::Lpp as LppTrait;

use crate::error::ContractError;
use crate::loan::Loan;

pub(crate) struct Result {
    pub batch: Batch,
    pub annual_interest_rate: Percent,
    pub annual_interest_rate_margin: Percent,
    pub loan_pool_id: Addr,
}

impl Result
{
    pub(super) fn try_new<Lpp, Lpn>(loan: Loan<Lpn, Lpp>, lease: impl Into<Addr>) -> StdResult<Self, ContractError>
    where
        Lpp: LppTrait<Lpn>,
        Lpn: Currency,
    {
        Ok(Self {
            annual_interest_rate: loan.annual_interest(lease)?,
            annual_interest_rate_margin: loan.annual_interest_margin(),
            loan_pool_id: loan.lpp.id(),
            batch: loan.lpp.into(),
        })
    }
}
