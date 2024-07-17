use currency::{Currency, CurrencyDef, MemberOf};
use finance::liability::Zone;
use lpp::stub::loan::LppLoan as LppLoanTrait;
use oracle_platform::Oracle as OracleTrait;
use platform::batch::Batch;
use sdk::cosmwasm_std::Timestamp;
use timealarms::stub::TimeAlarmsRef;

use crate::{
    api::{LeaseAssetCurrencies, LeasePaymentCurrencies},
    error::ContractResult,
    finance::{LpnCurrencies, LpnCurrency, OracleRef, Price},
    position::{Debt, DueTrait, Liquidation},
};

use super::Lease;

impl<Asset, Lpp, Oracle> Lease<Asset, Lpp, Oracle>
where
    Asset: CurrencyDef,
    Asset::Group: MemberOf<LeaseAssetCurrencies> + MemberOf<LeasePaymentCurrencies>,
    Lpp: LppLoanTrait<LpnCurrency, LpnCurrencies>,
    Oracle: OracleTrait<LeasePaymentCurrencies, QuoteC = LpnCurrency, QuoteG = LpnCurrencies>,
{
    pub(crate) fn check_debt(
        &self,
        now: &Timestamp,
        time_alarms: &TimeAlarmsRef,
        price_alarms: &OracleRef,
    ) -> ContractResult<DebtStatus<Asset>> {
        self.loan.state(now).and_then(|due| {
            self.price_of_lease_currency()
                .and_then(|asset_in_lpns| self.position.debt(&due, asset_in_lpns))
                .and_then(|debt| match debt {
                    Debt::No => Ok(DebtStatus::NoDebt),
                    Debt::Ok { zone, recheck_in } => self
                        .reschedule(
                            now,
                            recheck_in,
                            &zone,
                            due.total_due(),
                            time_alarms,
                            price_alarms,
                        )
                        .map(|alarms| DebtStatus::NewAlarms {
                            alarms,
                            current_liability: zone,
                        }),
                    Debt::Bad(liquidation) => Ok(DebtStatus::NeedLiquidation(liquidation)),
                })
        })
    }

    pub(super) fn price_of_lease_currency(&self) -> ContractResult<Price<Asset>> {
        self.oracle.price_of::<Asset>().map_err(Into::into)
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
