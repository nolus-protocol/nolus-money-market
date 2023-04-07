use serde::Serialize;

use finance::{coin::Coin, currency::Currency};
use lpp::stub::lender::{LppLender as LppLenderTrait, LppLenderRef};
use oracle::stub::{Oracle as OracleTrait, OracleRef};
use profit::stub::{Profit as ProfitTrait, ProfitRef};
use sdk::cosmwasm_std::{Addr, QuerierWrapper, Timestamp};
use timealarms::stub::{TimeAlarms as TimeAlarmsTrait, TimeAlarmsRef};

use crate::{
    api::{LeaseCoin, NewLeaseForm},
    error::{ContractError, ContractResult},
    lease::{
        with_lease_deps::{self, WithLeaseDeps},
        IntoDTOResult, Lease,
    },
    loan::Loan,
};

impl NewLeaseForm {
    pub(crate) fn into_lease(
        self,
        lease_addr: Addr,
        start_at: Timestamp,
        amount: &LeaseCoin,
        querier: &QuerierWrapper<'_>,
        deps: (LppLenderRef, OracleRef, TimeAlarmsRef),
    ) -> ContractResult<IntoDTOResult> {
        debug_assert_eq!(amount.ticker(), &self.currency);
        debug_assert!(amount.amount() > 0);

        let profit = ProfitRef::new(self.loan.profit.clone(), querier)?;

        let cmd = LeaseFactory {
            form: self,
            lease_addr,
            start_at,
            amount,
        };
        //TODO avoid cloning by extending the trait WithLeaseDeps to provide it
        let asset_currency = cmd.form.currency.clone();
        with_lease_deps::execute(
            cmd,
            &asset_currency,
            deps.0,
            profit,
            deps.2,
            deps.1,
            querier,
        )
    }
}

struct LeaseFactory<'a> {
    form: NewLeaseForm,
    lease_addr: Addr,
    start_at: Timestamp,
    amount: &'a LeaseCoin,
}

impl<'a> WithLeaseDeps for LeaseFactory<'a> {
    type Output = IntoDTOResult;
    type Error = ContractError;

    fn exec<Lpn, Asset, Lpp, Profit, TimeAlarms, Oracle>(
        self,
        lpp: Lpp,
        profit: Profit,
        alarms: TimeAlarms,
        oracle: Oracle,
    ) -> Result<Self::Output, Self::Error>
    where
        Lpn: Currency + Serialize,
        Asset: Currency + Serialize,
        Lpp: LppLenderTrait<Lpn>,
        TimeAlarms: TimeAlarmsTrait,
        Oracle: OracleTrait<Lpn>,
        Profit: ProfitTrait,
    {
        let liability = self.form.liability;

        let loan = Loan::new(
            self.start_at,
            lpp,
            self.form.loan.annual_margin_interest,
            self.form.loan.interest_payment,
            profit,
        );
        let amount: Coin<Asset> = self.amount.try_into()?;

        Ok(Lease::<_, Asset, _, _, _, _>::new(
            self.lease_addr,
            self.form.customer,
            amount,
            self.start_at,
            liability,
            loan,
            (alarms, oracle),
        )?
        .into_dto())
    }
}
