use currency::{Currency, CurrencyDef, MemberOf};
use finance::liability::Zone;
use lpp::stub::loan::LppLoan as LppLoanTrait;
use oracle_platform::Oracle as OracleTrait;
use sdk::cosmwasm_std::Timestamp;

use crate::{
    api::{LeaseAssetCurrencies, LeasePaymentCurrencies, position::ClosePolicyChange},
    error::{ContractError, ContractResult},
    finance::{LpnCurrencies, LpnCurrency, Price},
    position::{CloseStrategy, Debt, Liquidation, Steadiness},
};

use super::Lease;

impl<Asset, Lpp, Oracle> Lease<Asset, Lpp, Oracle>
where
    Asset: CurrencyDef,
    Asset::Group: MemberOf<LeaseAssetCurrencies> + MemberOf<LeasePaymentCurrencies>,
    Lpp: LppLoanTrait<LpnCurrency, LpnCurrencies>,
    Oracle: OracleTrait<LeasePaymentCurrencies, QuoteC = LpnCurrency, QuoteG = LpnCurrencies>,
{
    /// Check if the position requires
    /// - partial or full close due to a bad dept, or
    /// - full close due to a Stop-Loss or Take-Profit trigger.
    pub(crate) fn check_close_policy(&self, now: &Timestamp) -> ContractResult<CloseStatus<Asset>> {
        let due = self.loan.state(now);

        self.price_of_lease_currency().map(|asset_in_lpns| {
            self.position
                .check_close(&due, asset_in_lpns)
                .map(|close| CloseStatus::CloseAsked(close))
                .unwrap_or_else(|| match self.position.debt(&due, asset_in_lpns) {
                    Debt::No => CloseStatus::Paid,
                    Debt::Ok { zone, steadiness } => CloseStatus::None {
                        current_liability: zone,
                        steadiness,
                    },
                    Debt::Bad(liquidation) => CloseStatus::NeedLiquidation(liquidation),
                })
        })
    }

    pub(crate) fn change_close_policy(
        &mut self,
        cmd: ClosePolicyChange,
        now: &Timestamp,
    ) -> ContractResult<()> {
        let due = self.loan.state(now);

        self.price_of_lease_currency().and_then(|asset_in_lpns| {
            self.position
                .change_close_policy(cmd, &due, asset_in_lpns)
                .map_err(Into::into)
        })
    }

    pub(super) fn price_of_lease_currency(&self) -> ContractResult<Price<Asset>> {
        self.oracle
            .price_of::<Asset>()
            .map_err(ContractError::FetchOraclePrice)
    }
}

pub(crate) enum CloseStatus<Asset>
where
    Asset: Currency,
{
    Paid,
    None {
        current_liability: Zone,
        steadiness: Steadiness<Asset>,
    },
    CloseAsked(CloseStrategy),
    NeedLiquidation(Liquidation<Asset>),
}
