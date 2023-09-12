use currency::Currency;
use lpp::stub::loan::LppLoan as LppLoanTrait;
use oracle::stub::{Oracle as OracleTrait, OracleRef};
use profit::stub::ProfitRef;
use sdk::cosmwasm_std::Timestamp;
use timealarms::stub::TimeAlarmsRef;

use crate::{
    api::{LeaseCoin, LpnCoin},
    error::ContractError,
    lease::{with_lease::WithLease, IntoDTOResult, Lease},
};

use super::{liquidation_status, RepayResult};

pub(crate) type LiquidateResult = RepayResult;

pub(crate) struct Liquidate {
    asset: LeaseCoin,
    payment: LpnCoin,
    now: Timestamp,
    profit: ProfitRef,
    time_alarms: TimeAlarmsRef,
    price_alarms: OracleRef,
}

impl Liquidate {
    pub fn new(
        asset: LeaseCoin,
        payment: LpnCoin,
        now: Timestamp,
        profit: ProfitRef,
        time_alarms: TimeAlarmsRef,
        price_alarms: OracleRef,
    ) -> Self {
        Self {
            asset,
            payment,
            now,
            profit,
            time_alarms,
            price_alarms,
        }
    }
}

impl WithLease for Liquidate {
    type Output = LiquidateResult;

    type Error = ContractError;

    fn exec<Lpn, Asset, Lpp, Oracle>(
        self,
        mut lease: Lease<Lpn, Asset, Lpp, Oracle>,
    ) -> Result<Self::Output, Self::Error>
    where
        Lpn: Currency,
        Lpp: LppLoanTrait<Lpn>,
        Oracle: OracleTrait<Lpn>,
        Asset: Currency,
    {
        let mut profit = self.profit.as_stub();
        let receipt = lease.close_partial(
            self.asset.try_into()?,
            self.payment.try_into()?,
            self.now,
            &mut profit,
        )?;

        let liquidation = liquidation_status::status_and_schedule(
            &lease,
            self.now,
            &self.time_alarms,
            &self.price_alarms,
        )?;

        lease.try_into_dto(self.profit, self.time_alarms).map(
            |IntoDTOResult {
                 lease,
                 batch: messages,
             }| Self::Output {
                lease,
                receipt: receipt.into(),
                messages: messages.merge(profit.into()),
                liquidation,
            },
        )
    }
}
