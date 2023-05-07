use serde::Serialize;

use finance::{coin::Coin, currency::Currency};
use lpp::stub::{loan::LppLoan as LppLoanTrait, LppRef};
use oracle::stub::{Oracle as OracleTrait, OracleRef};
use profit::stub::{Profit as ProfitTrait, ProfitRef};
use sdk::cosmwasm_std::{Addr, QuerierWrapper, Timestamp};
use timealarms::stub::TimeAlarmsRef;

use crate::{
    api::{LeaseCoin, NewLeaseForm},
    error::{ContractError, ContractResult},
    lease::{
        with_lease_deps::{self, WithLeaseDeps},
        IntoDTOResult, Lease,
    },
    loan::Loan,
};

use super::{liquidation_status, LiquidationStatus};

pub(crate) fn open_lease(
    form: NewLeaseForm,
    lease_addr: Addr,
    start_at: Timestamp,
    amount: &LeaseCoin,
    querier: &QuerierWrapper<'_>,
    deps: (LppRef, OracleRef),
) -> ContractResult<IntoDTOResult> {
    debug_assert_eq!(amount.ticker(), &form.currency);
    debug_assert!(amount.amount() > 0);

    let time_alarms = TimeAlarmsRef::new(form.time_alarms.clone(), querier)?;
    let profit = ProfitRef::new(form.loan.profit.clone(), querier)?;

    let cmd = LeaseFactory {
        form,
        lease_addr: lease_addr.clone(),
        time_alarms,
        price_alarms: deps.1.clone(),
        start_at,
        amount,
    };
    //TODO avoid cloning by extending the trait WithLeaseDeps to provide it
    let asset_currency = cmd.form.currency.clone();
    with_lease_deps::execute(
        cmd,
        lease_addr,
        &asset_currency,
        deps.0,
        profit,
        deps.1,
        querier,
    )
}

struct LeaseFactory<'a> {
    form: NewLeaseForm,
    lease_addr: Addr,
    time_alarms: TimeAlarmsRef,
    price_alarms: OracleRef,
    start_at: Timestamp,
    amount: &'a LeaseCoin,
}

impl<'a> WithLeaseDeps for LeaseFactory<'a> {
    type Output = IntoDTOResult;
    type Error = ContractError;

    fn exec<Lpn, Asset, LppLoan, Profit, Oracle>(
        self,
        lpp_loan: Option<LppLoan>,
        profit: Profit,
        oracle: Oracle,
    ) -> Result<Self::Output, Self::Error>
    where
        Lpn: Currency + Serialize,
        Asset: Currency + Serialize,
        LppLoan: LppLoanTrait<Lpn>,
        Oracle: OracleTrait<Lpn>,
        Profit: ProfitTrait,
    {
        debug_assert!(lpp_loan.is_some());
        let liability = self.form.liability;

        let loan = Loan::new(
            self.start_at,
            lpp_loan,
            self.form.loan.annual_margin_interest,
            self.form.loan.interest_payment,
            profit,
        );
        let amount: Coin<Asset> = self.amount.try_into()?;

        let lease = Lease::<_, Asset, _, _, _>::new(
            self.lease_addr,
            self.form.customer,
            amount,
            liability,
            loan,
            oracle,
        );

        let alarms = match liquidation_status::status_and_schedule(
            &lease,
            self.start_at,
            &self.time_alarms,
            &self.price_alarms,
        )? {
            LiquidationStatus::NewAlarms {
                current_liability: _,
                alarms,
            } => alarms,
            LiquidationStatus::NeedLiquidation(_) => unreachable!(),
        };

        let mut dto = lease.into_dto(self.time_alarms);
        dto.batch = dto.batch.merge(alarms);
        Ok(dto)
    }
}
