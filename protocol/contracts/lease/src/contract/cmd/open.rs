use currency::Currency;
use lpp::stub::loan::LppLoan as LppLoanTrait;
use oracle_platform::Oracle as OracleTrait;
use profit::stub::ProfitRef;
use sdk::cosmwasm_std::{Addr, QuerierWrapper, Timestamp};
use timealarms::stub::TimeAlarmsRef;

use crate::{
    api::{open::NewLeaseForm, LeaseCoin},
    error::{ContractError, ContractResult},
    finance::{LpnCurrencies, LpnCurrency, LppRef, OracleRef, ReserveRef},
    lease::{
        with_lease_deps::{self, WithLeaseDeps},
        IntoDTOResult, Lease,
    },
    loan::Loan,
    position::{Position, Spec as PositionSpec},
};

use super::{check_debt, LiquidationStatus};

pub(crate) fn open_lease(
    form: NewLeaseForm,
    lease_addr: Addr,
    start_at: Timestamp,
    now: &Timestamp,
    amount: LeaseCoin,
    querier: QuerierWrapper<'_>,
    deps: (LppRef, OracleRef, TimeAlarmsRef),
) -> ContractResult<IntoDTOResult> {
    debug_assert_eq!(amount.ticker(), &form.currency);
    debug_assert!(amount.amount() > 0);

    let profit = ProfitRef::new(form.loan.profit.clone(), &querier)?;
    let reserve = ReserveRef::try_new(form.reserve.clone(), &querier)?;

    let cmd = LeaseFactory {
        form,
        lease_addr: lease_addr.clone(),
        profit,
        reserve,
        time_alarms: deps.2,
        price_alarms: deps.1.clone(),
        start_at,
        now,
        amount,
    };
    //TODO avoid cloning by extending the trait WithLeaseDeps to provide it
    let asset_currency = cmd.form.currency.clone();
    with_lease_deps::execute(cmd, lease_addr, &asset_currency, deps.0, deps.1, querier)
}

struct LeaseFactory<'a> {
    form: NewLeaseForm,
    lease_addr: Addr,
    profit: ProfitRef,
    reserve: ReserveRef,
    time_alarms: TimeAlarmsRef,
    price_alarms: OracleRef,
    start_at: Timestamp,
    now: &'a Timestamp,
    amount: LeaseCoin,
}

impl<'a> WithLeaseDeps for LeaseFactory<'a> {
    type Output = IntoDTOResult;
    type Error = ContractError;

    fn exec<Lpn, Asset, LppLoan, Oracle>(
        self,
        lpp_loan: LppLoan,
        oracle: Oracle,
    ) -> Result<Self::Output, Self::Error>
    where
        Lpn: ?Sized,
        Asset: Currency,
        LppLoan: LppLoanTrait<LpnCurrency, LpnCurrencies>,
        Oracle: OracleTrait<LpnCurrency>,
    {
        let lease = PositionSpec::try_from(self.form.position_spec)
            .and_then(|spec| Position::<Asset>::try_from(self.amount, spec))
            .map(|position| {
                let loan = Loan::new(
                    lpp_loan,
                    self.start_at,
                    self.form.loan.annual_margin_interest,
                    self.form.loan.due_period,
                );
                Lease::<Asset, _, _>::new(
                    self.lease_addr,
                    self.form.customer,
                    position,
                    loan,
                    oracle,
                )
            })?;

        let alarms = match check_debt::check_debt(
            &lease,
            self.now,
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
            .try_into_dto(self.profit, self.time_alarms, self.reserve)
            .map(|mut dto| {
                dto.batch = dto.batch.merge(alarms);
                dto
            })
    }
}
