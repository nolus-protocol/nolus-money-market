use currency::{CurrencyDef, MemberOf};

use crate::{
    api::{
        query::{opened, paid, StateResponse},
        LeaseAssetCurrencies,
    },
    lease::{LeaseDTO, State},
};

impl StateResponse {
    pub fn opened_from<Asset>(
        open_lease: State<Asset>,
        in_progress: Option<opened::OngoingTrx>,
    ) -> Self
    where
        Asset: CurrencyDef,
        Asset::Group: MemberOf<LeaseAssetCurrencies>,
    {
        Self::Opened {
            amount: open_lease.amount.into(),
            loan_interest_rate: open_lease.interest_rate,
            margin_interest_rate: open_lease.interest_rate_margin,
            principal_due: open_lease.principal_due.into(),
            overdue_margin: open_lease.overdue_margin.into(),
            overdue_interest: open_lease.overdue_interest.into(),
            overdue_collect_in: open_lease.overdue_collect_in,
            due_margin: open_lease.due_margin.into(),
            due_interest: open_lease.due_interest.into(),
            close_policy: open_lease.close_policy,
            validity: open_lease.validity,
            in_progress,
        }
    }

    pub fn paid_from(lease: LeaseDTO, in_progress: Option<paid::ClosingTrx>) -> Self {
        Self::Paid {
            amount: lease.position.into(),
            in_progress,
        }
    }
}
