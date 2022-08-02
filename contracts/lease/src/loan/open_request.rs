use cosmwasm_std::Addr;
use finance::{
    currency::Currency,
    percent::Percent
};
use platform::batch::Batch;
use lpp::stub::Lpp as LppTrait;

use crate::loan::Loan;

pub(crate) struct Result {
    pub batch: Batch,
    pub annual_interest: Percent,
    pub loan_pool_id: Addr,
}

impl<Lpp, Lpn> From<Loan<Lpn, Lpp>> for Result
where
    Lpp: LppTrait<Lpn>,
    Lpn: Currency,
{
    fn from(loan: Loan<Lpn, Lpp>) -> Self {
        Self {
            annual_interest: loan.annual_interest(),
            loan_pool_id: loan.lpp.id(),
            batch: loan.lpp.into(),
        }
    }
}
