use std::marker::PhantomData;

use cosmwasm_std::{Addr, QuerierWrapper};
use serde::{de::DeserializeOwned, Serialize};

use finance::currency::{visit_any, AnyVisitor, Currency, SymbolOwned};
use lpp::stub::{Lpp as LppTrait, WithLpp};
use market_price_oracle::stub::{Oracle as OracleTrait, OracleRef, WithOracle};
use time_alarms::stub::{TimeAlarms as TimeAlarmsTrait, TimeAlarmsRef, WithTimeAlarms};

use super::{dto::LeaseDTO, Lease, WithLease};

pub struct Factory<'r, L> {
    cmd: L,
    lease_dto: LeaseDTO,
    lease_addr: &'r Addr,
    querier: &'r QuerierWrapper<'r>,
}

impl<'r, L> Factory<'r, L> {
    pub fn new(
        cmd: L,
        lease_dto: LeaseDTO,
        lease_addr: &'r Addr,
        querier: &'r QuerierWrapper<'r>,
    ) -> Self {
        Self {
            cmd,
            lease_dto,
            lease_addr,
            querier,
        }
    }
}

impl<'r, L, O, E> WithLpp for Factory<'r, L>
where
    L: WithLease<Output = O, Error = E>,
{
    type Output = O;
    type Error = E;

    fn exec<Lpn, Lpp>(self, lpp: Lpp) -> Result<Self::Output, Self::Error>
    where
        Lpn: Currency + Serialize,
        Lpp: LppTrait<Lpn>,
    {
        let time_alarms = TimeAlarmsRef::try_from(self.lease_dto.time_alarms.clone())
            .expect("Time Alarms is not deployed, or wrong address is passed!");

        time_alarms.execute(FactoryStage2 {
            cmd: self.cmd,
            lease_dto: self.lease_dto,
            lease_addr: self.lease_addr,
            _lpn: PhantomData,
            lpp,
            querier: self.querier,
        })
    }

    fn unknown_lpn(self, symbol: SymbolOwned) -> Result<Self::Output, Self::Error> {
        self.cmd.unknown_lpn(symbol)
    }
}

struct FactoryStage2<'r, L, Lpn, Lpp> {
    cmd: L,
    lease_dto: LeaseDTO,
    lease_addr: &'r Addr,
    _lpn: PhantomData<Lpn>,
    lpp: Lpp,
    querier: &'r QuerierWrapper<'r>,
}

impl<'r, L, Lpn, Lpp> WithTimeAlarms for FactoryStage2<'r, L, Lpn, Lpp>
where
    L: WithLease,
    Lpn: Currency + Serialize,
    Lpp: LppTrait<Lpn>,
{
    type Output = L::Output;
    type Error = L::Error;

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
                lpp: self.lpp,
                time_alarms,
            },
            self.querier,
        )
    }
}

struct FactoryStage3<'r, L, Lpp, TimeAlarms> {
    cmd: L,
    lease_dto: LeaseDTO,
    lease_addr: &'r Addr,
    lpp: Lpp,
    time_alarms: TimeAlarms,
}

impl<'r, L, Lpn, Lpp, TimeAlarms> WithOracle<Lpn> for FactoryStage3<'r, L, Lpp, TimeAlarms>
where
    L: WithLease,
    Lpn: Currency + Serialize,
    Lpp: LppTrait<Lpn>,
    TimeAlarms: TimeAlarmsTrait,
{
    type Output = L::Output;
    type Error = L::Error;

    fn exec<Oracle>(self, oracle: Oracle) -> Result<Self::Output, Self::Error>
    where
        Oracle: OracleTrait<Lpn>,
    {
        visit_any(
            &self.lease_dto.currency.clone(),
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

    fn unexpected_base(self, symbol: SymbolOwned) -> Result<Self::Output, Self::Error> {
        self.cmd.unknown_lpn(symbol)
    }
}

struct FactoryStage4<'r, L, Lpn, Lpp, TimeAlarms, Oracle> {
    cmd: L,
    _lpn: PhantomData<Lpn>,
    lease_dto: LeaseDTO,
    lease_addr: &'r Addr,
    lpp: Lpp,
    time_alarms: TimeAlarms,
    oracle: Oracle,
}

impl<'r, L, Lpn, Lpp, TimeAlarms, Oracle> AnyVisitor
    for FactoryStage4<'r, L, Lpn, Lpp, TimeAlarms, Oracle>
where
    L: WithLease,
    Lpn: Currency + Serialize,
    Lpp: LppTrait<Lpn>,
    TimeAlarms: TimeAlarmsTrait,
    Oracle: OracleTrait<Lpn>,
{
    type Output = L::Output;
    type Error = L::Error;

    fn on<C>(self) -> Result<Self::Output, Self::Error>
    where
        C: 'static + Currency + Serialize + DeserializeOwned,
    {
        self.cmd.exec(Lease::<_, _, _, _, C>::from_dto(
            self.lease_dto,
            self.lease_addr,
            self.lpp,
            self.time_alarms,
            self.oracle,
        ))
    }

    fn on_unknown(self) -> Result<Self::Output, Self::Error> {
        self.cmd.unknown_lpn(self.lease_dto.currency)
    }
}
