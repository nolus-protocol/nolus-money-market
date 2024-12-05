use profit::stub::ProfitRef;
use serde::{Deserialize, Serialize};

use currency::{CurrencyDef, MemberOf};
use finance::liability::Zone;
use lpp::stub::loan::LppLoan as LppLoanTrait;
use oracle_platform::Oracle as OracleTrait;
use platform::batch::Batch;
use sdk::cosmwasm_std::Timestamp;
use timealarms::stub::TimeAlarmsRef;

use crate::{
    api::{position::ClosePolicyChange, LeaseAssetCurrencies, LeaseCoin, LeasePaymentCurrencies},
    contract::SplitDTOOut,
    error::{ContractError, ContractResult},
    finance::{LpnCurrencies, LpnCurrency, OracleRef, ReserveRef},
    lease::{with_lease::WithLease, CloseStatus, IntoDTOResult, Lease as LeaseDO, LeaseDTO},
    position::{Cause, CloseStrategy, Liquidation},
};

pub(crate) fn check<Asset, Lpp, Oracle>(
    lease: &LeaseDO<Asset, Lpp, Oracle>,
    when: &Timestamp,
    time_alarms: &TimeAlarmsRef,
    price_alarms: &OracleRef,
) -> ContractResult<CloseStatusDTO>
where
    Asset: CurrencyDef,
    Asset::Group: MemberOf<LeaseAssetCurrencies> + MemberOf<LeasePaymentCurrencies>,
    Lpp: LppLoanTrait<LpnCurrency, LpnCurrencies>,
    Oracle: OracleTrait<LeasePaymentCurrencies, QuoteC = LpnCurrency, QuoteG = LpnCurrencies>,
{
    lease
        .check_close_policy(when)
        .and_then(|status| CloseStatusDTO::try_from_do(status, when, time_alarms, price_alarms))
}

pub(crate) struct CheckCmd<'a> {
    now: &'a Timestamp,
    time_alarms: &'a TimeAlarmsRef,
    price_alarms: &'a OracleRef,
}

pub(crate) struct ChangeCmd<'a> {
    change: ClosePolicyChange,
    now: &'a Timestamp,
    // LeaseDTO attributes
    profit: ProfitRef,
    reserve: ReserveRef,
    time_alarms: TimeAlarmsRef,
}

pub(crate) enum CloseStatusDTO {
    Paid,
    None {
        current_liability: Zone,
        alarms: Batch,
    },
    NeedLiquidation(LiquidationDTO),
    CloseAsked(CloseStrategy),
}

#[derive(Serialize, Deserialize)]
pub(crate) enum LiquidationDTO {
    Partial(PartialLiquidationDTO),
    Full(FullLiquidationDTO),
}

#[derive(Serialize, Deserialize)]
pub(crate) struct PartialLiquidationDTO {
    pub amount: LeaseCoin,
    pub cause: Cause,
}
#[derive(Serialize, Deserialize)]
pub(crate) struct FullLiquidationDTO {
    pub cause: Cause,
}

impl CloseStatusDTO {
    fn try_from_do<Asset>(
        status: CloseStatus<Asset>,
        when: &Timestamp,
        time_alarms: &TimeAlarmsRef,
        price_alarms: &OracleRef,
    ) -> ContractResult<Self>
    where
        Asset: CurrencyDef,
        Asset::Group: MemberOf<LeaseAssetCurrencies>,
    {
        match status {
            CloseStatus::Paid => Ok(Self::Paid),
            CloseStatus::None {
                current_liability,
                steadiness,
            } => steadiness
                .try_into_alarms(when, time_alarms, price_alarms)
                .map(|alarms| Self::None {
                    current_liability,
                    alarms,
                }),
            CloseStatus::NeedLiquidation(liquidation) => {
                Ok(Self::NeedLiquidation(liquidation.into()))
            }
            CloseStatus::CloseAsked(strategy) => Ok(Self::CloseAsked(strategy)),
        }
    }
}

impl<Asset> From<Liquidation<Asset>> for LiquidationDTO
where
    Asset: CurrencyDef,
    Asset::Group: MemberOf<LeaseAssetCurrencies>,
{
    fn from(value: Liquidation<Asset>) -> Self {
        match value {
            Liquidation::Partial { amount, cause } => Self::Partial(PartialLiquidationDTO {
                amount: amount.into(),
                cause,
            }),
            Liquidation::Full(cause) => Self::Full(FullLiquidationDTO { cause }),
        }
    }
}

impl<'a> CheckCmd<'a> {
    pub fn new(
        now: &'a Timestamp,
        time_alarms: &'a TimeAlarmsRef,
        price_alarms: &'a OracleRef,
    ) -> Self {
        Self {
            now,
            time_alarms,
            price_alarms,
        }
    }
}

impl WithLease for CheckCmd<'_> {
    type Output = CloseStatusDTO;

    type Error = ContractError;

    fn exec<Asset, Loan, Oracle>(
        self,
        lease: LeaseDO<Asset, Loan, Oracle>,
    ) -> Result<Self::Output, Self::Error>
    where
        Asset: CurrencyDef,
        Asset::Group: MemberOf<LeaseAssetCurrencies> + MemberOf<LeasePaymentCurrencies>,
        Loan: LppLoanTrait<LpnCurrency, LpnCurrencies>,
        Oracle: OracleTrait<LeasePaymentCurrencies, QuoteC = LpnCurrency, QuoteG = LpnCurrencies>,
    {
        check(&lease, self.now, self.time_alarms, self.price_alarms)
    }
}

impl<'a> ChangeCmd<'a> {
    pub fn new(
        change: ClosePolicyChange,
        now: &'a Timestamp,
        // LeaseDTO attributes follow
        profit: ProfitRef,
        time_alarms: TimeAlarmsRef,
        reserve: ReserveRef,
    ) -> Self {
        Self {
            change,
            now,
            profit,
            reserve,
            time_alarms,
        }
    }
}

impl WithLease for ChangeCmd<'_> {
    type Output = IntoDTOResult;

    type Error = ContractError;

    fn exec<Asset, Loan, Oracle>(
        self,
        mut lease: LeaseDO<Asset, Loan, Oracle>,
    ) -> Result<Self::Output, Self::Error>
    where
        Asset: CurrencyDef,
        Asset::Group: MemberOf<LeaseAssetCurrencies> + MemberOf<LeasePaymentCurrencies>,
        Loan: LppLoanTrait<LpnCurrency, LpnCurrencies>,
        Oracle: OracleTrait<LeasePaymentCurrencies, QuoteC = LpnCurrency, QuoteG = LpnCurrencies>
            + Into<OracleRef>,
    {
        lease
            .change_close_policy(self.change, self.now)
            .and_then(|()| {
                lease
                    .try_into_dto(self.profit, self.time_alarms, self.reserve)
                    .inspect(|res| {
                        debug_assert!(res.batch.is_empty());
                    })
            })
    }
}

impl SplitDTOOut for IntoDTOResult {
    type Other = Batch;

    fn split_into(self) -> (LeaseDTO, Self::Other) {
        (self.lease, self.batch)
    }
}
