use currency::{CurrencyDef, MemberOf};
use lpp::stub::loan::LppLoan as LppLoanTrait;
use oracle_platform::Oracle as OracleTrait;
use profit::stub::ProfitRef;
use sdk::cosmwasm_std::{Addr, Timestamp};
use timealarms::stub::TimeAlarmsRef;

use crate::{
    api::{LeaseAssetCurrencies, LeasePaymentCurrencies, open::NewLeaseForm},
    contract::LeaseDTOResult,
    error::ContractError,
    finance::{LpnCurrencies, LpnCurrency, OracleRef, ReserveRef},
    lease::{Lease, with_lease_deps::WithLeaseDeps},
    loan::Loan,
    position::Position,
};

use super::{CloseStatusDTO, close_policy::check};

pub struct LeaseFactory<'a> {
    form: NewLeaseForm,
    lease_addr: Addr,
    profit: ProfitRef,
    reserve: ReserveRef,
    time_alarms: TimeAlarmsRef,
    price_alarms: OracleRef,
    start_at: Timestamp,
    now: &'a Timestamp,
}

pub type OpenLeaseResult = LeaseDTOResult<CloseStatusDTO>;

impl<'a> LeaseFactory<'a> {
    pub(crate) fn new(
        form: NewLeaseForm,
        lease_addr: Addr,
        profit: ProfitRef,
        reserve: ReserveRef,
        alarms: (TimeAlarmsRef, OracleRef),
        start_at: Timestamp,
        now: &'a Timestamp,
    ) -> Self {
        Self {
            form,
            lease_addr,
            profit,
            reserve,
            time_alarms: alarms.0,
            price_alarms: alarms.1,
            start_at,
            now,
        }
    }
}

impl WithLeaseDeps for LeaseFactory<'_> {
    type Output = OpenLeaseResult;
    type Error = ContractError;

    fn exec<Lpn, Asset, LppLoan, Oracle>(
        self,
        position: Position<Asset>,
        lpp_loan: LppLoan,
        oracle: Oracle,
    ) -> Result<Self::Output, Self::Error>
    where
        Asset: CurrencyDef,
        Asset::Group: MemberOf<LeaseAssetCurrencies> + MemberOf<LeasePaymentCurrencies>,
        LppLoan: LppLoanTrait<LpnCurrency>,
        Oracle: OracleTrait<LeasePaymentCurrencies, QuoteC = LpnCurrency, QuoteG = LpnCurrencies>
            + Into<OracleRef>,
    {
        let lease = {
            let loan = Loan::new(
                lpp_loan,
                self.start_at,
                self.form.loan.annual_margin_interest,
                self.form.loan.due_period,
            );
            Lease::new(self.lease_addr, self.form.customer, position, loan, oracle)
        };

        check::check(&lease, self.now, &self.time_alarms, &self.price_alarms).map(|status| {
            OpenLeaseResult {
                lease: lease.into_dto(self.profit, self.time_alarms, self.reserve),
                result: status,
            }
        })
    }
}
