use currency::Currency;

use crate::{
    api::{opened, paid, StateResponse},
    lease::{LeaseDTO, State},
};

impl StateResponse {
    pub fn opened_from<Asset, Lpn>(
        open_lease: State<Asset, Lpn>,
        in_progress: Option<opened::OngoingTrx>,
    ) -> Self
    where
        Asset: Currency,
        Lpn: Currency,
    {
        Self::Opened {
            amount: open_lease.amount.into(),
            loan_interest_rate: open_lease.interest_rate,
            margin_interest_rate: open_lease.interest_rate_margin,
            principal_due: open_lease.principal_due.into(),
            previous_margin_due: open_lease.previous_margin_due.into(),
            previous_interest_due: open_lease.previous_interest_due.into(),
            current_margin_due: open_lease.current_margin_due.into(),
            current_interest_due: open_lease.current_interest_due.into(),
            validity: open_lease.validity,
            in_progress,
        }
    }

    pub fn paid_from(lease: LeaseDTO, in_progress: Option<paid::ClosingTrx>) -> Self {
        Self::Paid {
            amount: lease.position.amount,
            in_progress,
        }
    }
}
