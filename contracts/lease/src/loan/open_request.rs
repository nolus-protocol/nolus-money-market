use cosmwasm_std::Addr;
use finance::{currency::Currency, percent::Percent};

use lpp::stub::Lpp as LppTrait;
use platform::batch::Batch;

use crate::loan::Loan;

pub(crate) struct Result {
    pub batch: Batch,
    pub annual_interest_rate: Percent,
    pub annual_interest_rate_margin: Percent,
    pub loan_pool_id: Addr,
}

impl Result {
    pub(super) fn new<Lpp, Lpn>(loan: Loan<Lpn, Lpp>, annual_interest_rate: Percent) -> Self
    where
        Lpp: LppTrait<Lpn>,
        Lpn: Currency,
    {
        Self {
            annual_interest_rate,
            annual_interest_rate_margin: loan.annual_interest_margin(),
            loan_pool_id: loan.lpp.id(),
            batch: loan.lpp.into(),
        }
    }
}
