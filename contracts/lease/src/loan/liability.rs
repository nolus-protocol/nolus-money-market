use cosmwasm_std::{Addr, Timestamp};

use finance::{coin::Coin, currency::Currency, percent::Percent};
use lpp::stub::Lpp as LppTrait;

use crate::{
    error::{ContractError, ContractResult},
    loan::Loan,
};

impl<Lpn, Lpp> Loan<Lpn, Lpp>
where
    Lpn: Currency,
    Lpp: LppTrait<Lpn>,
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
                let overdue = state.previous_margin_interest_due + state.previous_interest_due;

                let total = state.principal_due
                    + overdue
                    + state.current_margin_interest_due
                    + state.current_interest_due;

                LiabilityStatus {
                    ltv: Percent::from_ratio(total, lease_lpn),
                    total,
                    overdue,
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
    pub total: Coin<Lpn>,
    pub overdue: Coin<Lpn>,
}
