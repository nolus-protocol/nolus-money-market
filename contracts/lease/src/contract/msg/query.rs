use finance::currency::Currency;

use crate::{
    api::{opened, paid, StateResponse},
    lease::State,
};

impl StateResponse {
    pub fn opened_from<Asset, Lpn>(
        lease_state: State<Asset, Lpn>,
        in_progress: Option<opened::OngoingTrx>,
    ) -> Self
    where
        Asset: Currency,
        Lpn: Currency,
    {
        if let State::Opened {
            amount,
            interest_rate,
            interest_rate_margin,
            principal_due,
            previous_margin_due,
            previous_interest_due,
            current_margin_due,
            current_interest_due,
            validity,
        } = lease_state
        {
            Self::Opened {
                amount: amount.into(),
                loan_interest_rate: interest_rate,
                margin_interest_rate: interest_rate_margin,
                principal_due: principal_due.into(),
                previous_margin_due: previous_margin_due.into(),
                previous_interest_due: previous_interest_due.into(),
                current_margin_due: current_margin_due.into(),
                current_interest_due: current_interest_due.into(),
                validity,
                in_progress,
            }
        } else {
            unreachable!();
        }
    }

    pub fn paid_from<Asset, Lpn>(
        lease_state: State<Asset, Lpn>,
        in_progress: Option<paid::ClosingTrx>,
    ) -> Self
    where
        Asset: Currency,
        Lpn: Currency,
    {
        if let State::Paid(amount) = lease_state {
            Self::Paid {
                amount: amount.into(),
                in_progress,
            }
        } else {
            unreachable!();
        }
    }
}
