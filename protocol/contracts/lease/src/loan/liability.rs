use currency::Currency;
use finance::coin::Coin;
use lpp::stub::loan::LppLoan as LppLoanTrait;
use sdk::cosmwasm_std::Timestamp;

use crate::{api::LpnCurrencies, loan::Loan};

impl<Lpn, Lpp> Loan<Lpn, Lpp>
where
    Lpn: Currency,
    Lpp: LppLoanTrait<Lpn, LpnCurrencies>,
{
    pub(crate) fn liability_status(&self, now: Timestamp) -> LiabilityStatus<Lpn> {
        let state = self.state(now);

        let overdue = state.overdue_margin_interest + state.overdue_interest;

        let total_due =
            state.principal_due + overdue + state.due_margin_interest + state.due_interest;

        debug_assert!(overdue <= total_due);
        LiabilityStatus { total_due, overdue }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub(crate) struct LiabilityStatus<Lpn>
where
    Lpn: Currency,
{
    pub total_due: Coin<Lpn>,
    pub overdue: Coin<Lpn>,
}
