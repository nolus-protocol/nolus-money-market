use finance::currency::Currency;

use crate::{api::StateResponse, lease::State};

impl<Asset, Lpn> From<State<Asset, Lpn>> for StateResponse
where
    Asset: Currency,
    Lpn: Currency,
{
    fn from(state: State<Asset, Lpn>) -> Self {
        match state {
            State::Opened {
                amount,
                interest_rate,
                interest_rate_margin,
                principal_due,
                previous_margin_due,
                previous_interest_due,
                current_margin_due,
                current_interest_due,
                validity,
            } => Self::Opened {
                amount: amount.into(),
                interest_rate,
                interest_rate_margin,
                principal_due: principal_due.into(),
                previous_margin_due: previous_margin_due.into(),
                previous_interest_due: previous_interest_due.into(),
                current_margin_due: current_margin_due.into(),
                current_interest_due: current_interest_due.into(),
                validity,
            },
            State::Paid(amount) => Self::Paid(amount.into()),
            State::Closed() => Self::Closed(),
        }
    }
}
