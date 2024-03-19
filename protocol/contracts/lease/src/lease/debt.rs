use currency::Currency;
use finance::liability::Zone;
use lpp::stub::loan::LppLoan as LppLoanTrait;
use oracle_platform::{Oracle as OracleTrait, OracleRef};
use platform::batch::Batch;
use sdk::cosmwasm_std::Timestamp;
use timealarms::stub::TimeAlarmsRef;

use crate::{
    api::LeaseAssetCurrencies,
    error::ContractResult,
    finance::{LpnCurrencies, LpnCurrency, Price},
    position::{Debt, DueTrait, Liquidation},
};

use super::Lease;

impl<Asset, Lpp, Oracle> Lease<Asset, Lpp, Oracle>
where
    Lpp: LppLoanTrait<LpnCurrency, LpnCurrencies>,
    Oracle: OracleTrait<LpnCurrency>,
    Asset: Currency,
{
    pub(crate) fn check_debt(
        &self,
        now: &Timestamp,
        time_alarms: &TimeAlarmsRef,
        price_alarms: &OracleRef,
    ) -> ContractResult<DebtStatus<Asset>> {
        let due = self.loan.state(now);

        let debt = self
            .price_of_lease_currency()
            .map(|asset_in_lpns| self.position.debt(&due, asset_in_lpns))?;
        Ok(match debt {
            Debt::No => DebtStatus::NoDebt,
            Debt::Ok { zone, recheck_in } => DebtStatus::NewAlarms {
                alarms: self.reschedule(
                    now,
                    recheck_in,
                    &zone,
                    due.total_due(),
                    time_alarms,
                    price_alarms,
                )?,
                current_liability: zone,
            },
            Debt::Bad(liquidation) => DebtStatus::NeedLiquidation(liquidation),
        })
    }

    pub(super) fn price_of_lease_currency(&self) -> ContractResult<Price<Asset>> {
        self.oracle
            .price_of::<Asset, LeaseAssetCurrencies>()
            .map_err(Into::into)
    }
}

pub(crate) enum DebtStatus<Asset>
where
    Asset: Currency,
{
    NoDebt,
    NewAlarms {
        current_liability: Zone,
        alarms: Batch,
    },
    NeedLiquidation(Liquidation<Asset>),
}
