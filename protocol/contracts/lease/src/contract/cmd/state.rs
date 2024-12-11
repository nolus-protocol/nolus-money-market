use currency::{CurrencyDef, MemberOf};
use finance::duration::Duration;
use lpp::stub::loan::LppLoan as LppLoanTrait;
use oracle_platform::Oracle as OracleTrait;
use sdk::cosmwasm_std::Timestamp;

use crate::{
    api::{
        query::{opened::OngoingTrx, StateResponse},
        LeaseAssetCurrencies, LeasePaymentCurrencies,
    },
    error::ContractError,
    finance::{LpnCurrencies, LpnCurrency},
    lease::{with_lease::WithLease, Lease},
};

pub struct LeaseState {
    now: Timestamp,
    due_projection: Duration,
    in_progress: Option<OngoingTrx>,
}

impl LeaseState {
    pub fn new(now: Timestamp, due_projection: Duration, in_progress: Option<OngoingTrx>) -> Self {
        Self {
            now,
            due_projection,
            in_progress,
        }
    }
}

impl WithLease for LeaseState {
    type Output = StateResponse;

    type Error = ContractError;

    fn exec<Asset, LppLoan, Oracle>(
        self,
        lease: Lease<Asset, LppLoan, Oracle>,
    ) -> Result<Self::Output, Self::Error>
    where
        Asset: CurrencyDef,
        Asset::Group: MemberOf<LeaseAssetCurrencies> + MemberOf<LeasePaymentCurrencies>,
        LppLoan: LppLoanTrait<LpnCurrency, LpnCurrencies>,
        Oracle: OracleTrait<LeasePaymentCurrencies, QuoteC = LpnCurrency, QuoteG = LpnCurrencies>,
    {
        Ok(StateResponse::opened_from(
            lease.state(self.now, self.due_projection),
            self.in_progress,
        ))
    }
}
