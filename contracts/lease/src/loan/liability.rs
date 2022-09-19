use cosmwasm_std::{Addr, Timestamp};

use finance::{coin::Coin, currency::Currency, percent::Percent};
use lpp::stub::lender::LppLender as LppLenderTrait;
use profit::stub::Profit as ProfitTrait;

use crate::{
    error::{ContractError, ContractResult},
    loan::Loan,
};

impl<Lpn, Lpp, Profit> Loan<Lpn, Lpp, Profit>
where
    Lpn: Currency,
    Lpp: LppLenderTrait<Lpn>,
    Profit: ProfitTrait,
{
    pub(crate) fn liability_status<A>(
        &self,
        now: Timestamp,
        lease: A,
        lease_lpn: Coin<Lpn>,
    ) -> ContractResult<LiabilityStatus<Lpn>>
    where
        A: Into<Addr>,
    {
        self.state(now, lease.into())?
            .map(|state| {
                let overdue_lpn = state.previous_margin_interest_due + state.previous_interest_due;

                let total_lpn = state.principal_due
                    + overdue_lpn
                    + state.current_margin_interest_due
                    + state.current_interest_due;

                LiabilityStatus {
                    ltv: Percent::from_ratio(total_lpn, lease_lpn),
                    total_lpn,
                    overdue_lpn,
                }
            })
            .ok_or(ContractError::LoanClosed())
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub(crate) struct LiabilityStatus<Lpn>
where
    Lpn: Currency,
{
    pub ltv: Percent,
    pub total_lpn: Coin<Lpn>,
    pub overdue_lpn: Coin<Lpn>,
}
