use std::marker::PhantomData;

use cosmwasm_std::{Addr, QuerierWrapper};
use serde::{de::DeserializeOwned, Serialize};

use currency::lease::LeaseGroup;
use finance::currency::{visit_any, AnyVisitor, Currency};
use lpp::stub::lender::{LppLender as LppLenderTrait, WithLppLender};
use market_price_oracle::stub::{Oracle as OracleTrait, OracleRef, WithOracle};
use platform::bank::FixedAddressSenderBuilder;
use profit::stub::{Profit as ProfitTrait, WithProfit};
use time_alarms::stub::{TimeAlarms as TimeAlarmsTrait, TimeAlarmsRef, WithTimeAlarms};

use super::{dto::LeaseDTO, Lease, WithLease};

pub struct Factory<'r, Cmd, SenderBuilder> {
    cmd: Cmd,
    lease_dto: LeaseDTO,
    lease_addr: &'r Addr,
    sender_builder: SenderBuilder,
    querier: &'r QuerierWrapper<'r>,
}

impl<'r, Cmd, SenderBuilder> Factory<'r, Cmd, SenderBuilder> {
    pub fn new(
        cmd: Cmd,
        lease_dto: LeaseDTO,
        lease_addr: &'r Addr,
        sender_builder: SenderBuilder,
        querier: &'r QuerierWrapper<'r>,
    ) -> Self {
        Self {
            cmd,
            lease_dto,
            lease_addr,
            sender_builder,
            querier,
        }
    }
}

impl<'r, Cmd, SenderBuilder> WithLppLender for Factory<'r, Cmd, SenderBuilder>
where
    Cmd: WithLease,
    finance::error::Error: Into<Cmd::Error>,
    time_alarms::error::ContractError: Into<Cmd::Error>,
    market_price_oracle::error::ContractError: Into<Cmd::Error>,
    profit::error::ContractError: Into<Cmd::Error>,
    SenderBuilder: FixedAddressSenderBuilder,
{
    type Output = Cmd::Output;
    type Error = Cmd::Error;

    fn exec<Lpn, Lpp>(self, lpp: Lpp) -> Result<Self::Output, Self::Error>
    where
        Lpn: Currency + Serialize,
        Lpp: LppLenderTrait<Lpn>,
    {
        let time_alarms = TimeAlarmsRef::try_from(self.lease_dto.time_alarms.clone())
            .expect("Time Alarms is not deployed, or wrong address is passed!");

        time_alarms.execute(FactoryStage2 {
            cmd: self.cmd,
            lease_dto: self.lease_dto,
            lease_addr: self.lease_addr,
            _lpn: PhantomData,
            lpp,
            sender_builder: self.sender_builder,
            querier: self.querier,
        })
    }
}

struct FactoryStage2<'r, Cmd, Lpn, Lpp, SenderBuilder> {
    cmd: Cmd,
    lease_dto: LeaseDTO,
    lease_addr: &'r Addr,
    _lpn: PhantomData<Lpn>,
    lpp: Lpp,
    sender_builder: SenderBuilder,
    querier: &'r QuerierWrapper<'r>,
}

impl<'r, Cmd, Lpn, Lpp, SenderBuilder> WithTimeAlarms
    for FactoryStage2<'r, Cmd, Lpn, Lpp, SenderBuilder>
where
    Cmd: WithLease,
    finance::error::Error: Into<Cmd::Error>,
    market_price_oracle::error::ContractError: Into<Cmd::Error>,
    profit::error::ContractError: Into<Cmd::Error>,
    Lpn: Currency + Serialize,
    Lpp: LppLenderTrait<Lpn>,
    SenderBuilder: FixedAddressSenderBuilder,
{
    type Output = Cmd::Output;
    type Error = Cmd::Error;

    fn exec<TimeAlarms>(self, time_alarms: TimeAlarms) -> Result<Self::Output, Self::Error>
    where
        TimeAlarms: TimeAlarmsTrait,
    {
        let oracle = OracleRef::try_from(self.lease_dto.oracle.clone(), self.querier)
            .expect("Market Price Oracle is not deployed, or wrong address is passed!");

        oracle.execute(
            FactoryStage3 {
                cmd: self.cmd,
                lease_dto: self.lease_dto,
                lease_addr: self.lease_addr,
                _lpn: PhantomData,
                lpp: self.lpp,
                sender_builder: self.sender_builder,
                time_alarms,
            },
            self.querier,
        )
    }
}

struct FactoryStage3<'r, Cmd, Lpn, Lpp, SenderBuilder, TimeAlarms> {
    cmd: Cmd,
    lease_dto: LeaseDTO,
    lease_addr: &'r Addr,
    _lpn: PhantomData<Lpn>,
    lpp: Lpp,
    sender_builder: SenderBuilder,
    time_alarms: TimeAlarms,
}

impl<'r, Cmd, Lpn, Lpp, SenderBuilder, TimeAlarms> WithOracle<Lpn>
    for FactoryStage3<'r, Cmd, Lpn, Lpp, SenderBuilder, TimeAlarms>
where
    Cmd: WithLease,
    finance::error::Error: Into<Cmd::Error>,
    profit::error::ContractError: Into<Cmd::Error>,
    Lpn: Currency + Serialize,
    Lpp: LppLenderTrait<Lpn>,
    SenderBuilder: FixedAddressSenderBuilder,
    TimeAlarms: TimeAlarmsTrait,
{
    type Output = Cmd::Output;
    type Error = Cmd::Error;

    fn exec<Oracle>(self, oracle: Oracle) -> Result<Self::Output, Self::Error>
    where
        Oracle: OracleTrait<Lpn>,
    {
        let profit = self.lease_dto.loan.profit().clone();

        profit.execute(
            self.sender_builder,
            FactoryStage4 {
                cmd: self.cmd,
                lease_dto: self.lease_dto,
                lease_addr: self.lease_addr,
                _lpn: PhantomData,
                lpp: self.lpp,
                time_alarms: self.time_alarms,
                oracle,
            },
        )
    }
}

struct FactoryStage4<'r, Cmd, Lpn, Lpp, TimeAlarms, Oracle> {
    cmd: Cmd,
    lease_dto: LeaseDTO,
    lease_addr: &'r Addr,
    _lpn: PhantomData<Lpn>,
    lpp: Lpp,
    time_alarms: TimeAlarms,
    oracle: Oracle,
}

impl<'r, Cmd, Lpn, Lpp, TimeAlarms, Oracle> WithProfit
    for FactoryStage4<'r, Cmd, Lpn, Lpp, TimeAlarms, Oracle>
where
    Cmd: WithLease,
    finance::error::Error: Into<Cmd::Error>,
    Lpn: Currency + Serialize,
    Lpp: LppLenderTrait<Lpn>,
    TimeAlarms: TimeAlarmsTrait,
    Oracle: OracleTrait<Lpn>,
{
    type Output = Cmd::Output;
    type Error = Cmd::Error;

    fn exec<P>(self, profit: P) -> Result<Self::Output, Self::Error>
    where
        P: ProfitTrait,
    {
        visit_any(
            &self.lease_dto.currency.clone(),
            FactoryStage5 {
                cmd: self.cmd,
                lease_dto: self.lease_dto,
                lease_addr: self.lease_addr,
                _lpn: PhantomData,
                lpp: self.lpp,
                profit,
                time_alarms: self.time_alarms,
                oracle: self.oracle,
            },
        )
    }
}

struct FactoryStage5<'r, Cmd, Lpn, Lpp, Profit, TimeAlarms, Oracle> {
    cmd: Cmd,
    lease_dto: LeaseDTO,
    lease_addr: &'r Addr,
    _lpn: PhantomData<Lpn>,
    lpp: Lpp,
    profit: Profit,
    time_alarms: TimeAlarms,
    oracle: Oracle,
}

impl<'r, L, Lpn, Lpp, Profit, TimeAlarms, Oracle> AnyVisitor<LeaseGroup>
    for FactoryStage5<'r, L, Lpn, Lpp, Profit, TimeAlarms, Oracle>
where
    L: WithLease,
    Lpn: Currency + Serialize,
    Lpp: LppLenderTrait<Lpn>,
    Profit: ProfitTrait,
    TimeAlarms: TimeAlarmsTrait,
    Oracle: OracleTrait<Lpn>,
{
    type Output = L::Output;
    type Error = L::Error;

    fn on<C>(self) -> Result<Self::Output, Self::Error>
    where
        C: 'static + Currency + Serialize + DeserializeOwned,
    {
        self.cmd.exec(Lease::<_, C, _, _, _, _>::from_dto(
            self.lease_dto,
            self.lease_addr,
            self.lpp,
            self.time_alarms,
            self.oracle,
            self.profit,
        ))
    }
}
