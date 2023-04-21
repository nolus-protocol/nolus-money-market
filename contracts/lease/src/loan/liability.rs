use finance::{coin::Coin, currency::Currency, percent::Percent};
use lpp::stub::lender::LppLender as LppLenderTrait;
use profit::stub::Profit as ProfitTrait;
use sdk::cosmwasm_std::{Addr, Timestamp};

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
                let previous_interest =
                    state.previous_margin_interest_due + state.previous_interest_due;

                let total = state.principal_due
                    + previous_interest
                    + state.current_margin_interest_due
                    + state.current_interest_due;

                LiabilityStatus {
                    ltv: Percent::from_ratio(total, lease_lpn),
                    total,
                    previous_interest,
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
    pub previous_interest: Coin<Lpn>,
}
