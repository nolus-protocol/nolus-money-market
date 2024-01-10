use currency::Currency;
use finance::coin::Coin;
use lpp::stub::loan::LppLoan as LppLoanTrait;
use sdk::cosmwasm_std::Timestamp;

use crate::loan::Loan;

impl<Lpn, Lpp> Loan<Lpn, Lpp>
where
    Lpn: Currency,
    Lpp: LppLoanTrait<Lpn>,
{
    pub(crate) fn liability_status(&self, now: Timestamp) -> LiabilityStatus<Lpn> {
        let state = self.state(now);

        let previous_interest = state.overdue_margin_interest + state.overdue_interest;

        let total = state.principal_due
            + previous_interest
            + state.due_margin_interest
            + state.due_interest;

        debug_assert!(previous_interest <= total);
        LiabilityStatus {
            total,
            previous_interest,
        }
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
