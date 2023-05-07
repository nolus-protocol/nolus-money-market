use finance::{coin::Coin, currency::Currency};
use lpp::stub::loan::LppLoan as LppLoanTrait;
use profit::stub::Profit as ProfitTrait;
use sdk::cosmwasm_std::Timestamp;

use crate::{
    error::{ContractError, ContractResult},
    loan::Loan,
};

impl<Lpn, Lpp, Profit> Loan<Lpn, Lpp, Profit>
where
    Lpn: Currency,
    Lpp: LppLoanTrait<Lpn>,
    Profit: ProfitTrait,
{
    pub(crate) fn liability_status(&self, now: Timestamp) -> ContractResult<LiabilityStatus<Lpn>> {
        self.state(now)?
            .map(|state| {
                let previous_interest =
                    state.previous_margin_interest_due + state.previous_interest_due;

                let total = state.principal_due
                    + previous_interest
                    + state.current_margin_interest_due
                    + state.current_interest_due;

                LiabilityStatus {
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
    pub total: Coin<Lpn>,
    pub previous_interest: Coin<Lpn>,
}
