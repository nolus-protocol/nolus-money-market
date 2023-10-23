use currency::Currency;
use lpp::stub::{loan::LppLoan as LppLoanTrait, LppRef};
use oracle_platform::{Oracle as OracleTrait, OracleRef};
use profit::stub::ProfitRef;
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
    position::Position,
};

use super::{liquidation_status, LiquidationStatus};

pub(crate) fn open_lease(
    form: NewLeaseForm,
    lease_addr: Addr,
    start_at: Timestamp,
    amount: LeaseCoin,
    querier: &QuerierWrapper<'_>,
    deps: (LppRef, OracleRef, TimeAlarmsRef),
) -> ContractResult<IntoDTOResult> {
    debug_assert_eq!(amount.ticker(), &form.currency);
    debug_assert!(amount.amount() > 0);

    let profit = ProfitRef::new(form.loan.profit.clone(), querier)?;

    let cmd = LeaseFactory {
        form,
        lease_addr: lease_addr.clone(),
        profit,
        time_alarms: deps.2,
        price_alarms: deps.1.clone(),
        start_at,
        amount,
    };
    //TODO avoid cloning by extending the trait WithLeaseDeps to provide it
    let asset_currency = cmd.form.currency.clone();
    with_lease_deps::execute(cmd, lease_addr, &asset_currency, deps.0, deps.1, querier)
}

struct LeaseFactory {
    form: NewLeaseForm,
    lease_addr: Addr,
    profit: ProfitRef,
    time_alarms: TimeAlarmsRef,
    price_alarms: OracleRef,
    start_at: Timestamp,
    amount: LeaseCoin,
}

impl WithLeaseDeps for LeaseFactory {
    type Output = IntoDTOResult;
    type Error = ContractError;

    fn exec<Lpn, Asset, LppLoan, Oracle>(
        self,
        lpp_loan: LppLoan,
        oracle: Oracle,
    ) -> Result<Self::Output, Self::Error>
    where
        Lpn: Currency,
        Asset: Currency,
        LppLoan: LppLoanTrait<Lpn>,
        Oracle: OracleTrait<Lpn>,
    {
        let position = Position::try_from(self.amount, self.form.position_spec)?;

        let loan = Loan::new(
            self.start_at,
            lpp_loan,
            self.form.loan.annual_margin_interest,
            self.form.loan.interest_payment,
        );

        let lease = Lease::<_, Asset, _, _>::new(
            self.lease_addr,
            self.form.customer,
            position,
            loan,
            oracle,
        );

        let alarms = match liquidation_status::status_and_schedule(
            &lease,
            self.start_at,
            &self.time_alarms,
            &self.price_alarms,
        )? {
            LiquidationStatus::NoDebt => unreachable!(),
            LiquidationStatus::NewAlarms {
                current_liability: _,
                alarms,
            } => alarms,
            LiquidationStatus::NeedLiquidation(_) => unreachable!(),
        };

        lease
            .try_into_dto(self.profit, self.time_alarms)
            .map(|mut dto| {
                dto.batch = dto.batch.merge(alarms);
                dto
            })
    }
}
