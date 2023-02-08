use std::marker::PhantomData;

use serde::{de::DeserializeOwned, Serialize};

use ::currency::lease::LeaseGroup;
use finance::currency::{self, AnyVisitor, AnyVisitorResult, Currency, Symbol};
use lpp::stub::lender::{LppLender as LppLenderTrait, LppLenderRef, WithLppLender};
use oracle::stub::{Oracle as OracleTrait, OracleRef, WithOracle};
use profit::stub::{Profit as ProfitTrait, ProfitRef, WithProfit};
use sdk::cosmwasm_std::QuerierWrapper;
use timealarms::stub::{TimeAlarms as TimeAlarmsTrait, TimeAlarmsRef, WithTimeAlarms};

pub trait WithLeaseDeps {
    type Output;
    type Error;

    fn exec<Lpn, Asset, Lpp, Profit, TimeAlarms, Oracle>(
        self,
        lpp: Lpp,
        profit: Profit,
        alarms: TimeAlarms,
        oracle: Oracle,
    ) -> Result<Self::Output, Self::Error>
    where
        Lpn: Currency + Serialize,
        Lpp: LppLenderTrait<Lpn>,
        TimeAlarms: TimeAlarmsTrait,
        Oracle: OracleTrait<Lpn>,
        Profit: ProfitTrait,
        Asset: Currency + Serialize;
}

pub fn execute<Cmd>(
    cmd: Cmd,
    asset: Symbol<'_>,
    lpp: LppLenderRef,
    profit: ProfitRef,
    alarms: TimeAlarmsRef,
    oracle: OracleRef,
    querier: &QuerierWrapper<'_>,
) -> Result<Cmd::Output, Cmd::Error>
where
    Cmd: WithLeaseDeps,
    finance::error::Error: Into<Cmd::Error>,
    oracle::error::ContractError: Into<Cmd::Error>,
    profit::error::ContractError: Into<Cmd::Error>,
{
    currency::visit_any_on_ticker::<LeaseGroup, _>(
        asset,
        FactoryStage1 {
            cmd,
            lpp,
            profit,
            alarms,
            oracle,
            querier,
        },
    )
}

struct FactoryStage1<'r, Cmd> {
    cmd: Cmd,
    lpp: LppLenderRef,
    profit: ProfitRef,
    oracle: OracleRef,
    alarms: TimeAlarmsRef,
    querier: &'r QuerierWrapper<'r>,
}

impl<'r, Cmd> AnyVisitor for FactoryStage1<'r, Cmd>
where
    Cmd: WithLeaseDeps,
    finance::error::Error: Into<Cmd::Error>,
    profit::error::ContractError: Into<Cmd::Error>,
    oracle::error::ContractError: Into<Cmd::Error>,
{
    type Output = Cmd::Output;
    type Error = Cmd::Error;

    fn on<C>(self) -> AnyVisitorResult<Self>
    where
        C: 'static + Currency + Serialize + DeserializeOwned,
    {
        self.lpp.execute(
            FactoryStage2 {
                cmd: self.cmd,
                asset: PhantomData::<C>,
                profit: self.profit,
                alarms: self.alarms,
                oracle: self.oracle,
                querier: self.querier,
            },
            self.querier,
        )
    }
}
struct FactoryStage2<'r, Cmd, Asset> {
    cmd: Cmd,
    asset: PhantomData<Asset>,
    profit: ProfitRef,
    oracle: OracleRef,
    alarms: TimeAlarmsRef,
    querier: &'r QuerierWrapper<'r>,
}

impl<'r, Cmd, Asset> WithLppLender for FactoryStage2<'r, Cmd, Asset>
where
    Cmd: WithLeaseDeps,
    Asset: Currency + Serialize,
    oracle::error::ContractError: Into<Cmd::Error>,
    profit::error::ContractError: Into<Cmd::Error>,
{
    type Output = Cmd::Output;
    type Error = Cmd::Error;

    fn exec<Lpn, Lpp>(self, lpp: Lpp) -> Result<Self::Output, Self::Error>
    where
        Lpn: Currency + Serialize,
        Lpp: LppLenderTrait<Lpn>,
    {
        self.profit.execute(FactoryStage3 {
            cmd: self.cmd,
            asset: self.asset,
            lpn: PhantomData::<Lpn>,
            lpp,
            oracle: self.oracle,
            alarms: self.alarms,
            querier: self.querier,
        })
    }
}

struct FactoryStage3<'r, Cmd, Asset, Lpn, Lpp> {
    cmd: Cmd,
    asset: PhantomData<Asset>,
    lpn: PhantomData<Lpn>,
    lpp: Lpp,
    oracle: OracleRef,
    alarms: TimeAlarmsRef,
    querier: &'r QuerierWrapper<'r>,
}

impl<'r, Cmd, Asset, Lpn, Lpp> WithProfit for FactoryStage3<'r, Cmd, Asset, Lpn, Lpp>
where
    Cmd: WithLeaseDeps,
    Asset: Currency + Serialize,
    Lpn: Currency + Serialize,
    Lpp: LppLenderTrait<Lpn>,
    oracle::error::ContractError: Into<Cmd::Error>,
{
    type Output = Cmd::Output;
    type Error = Cmd::Error;

    fn exec<P>(self, profit: P) -> Result<Self::Output, Self::Error>
    where
        P: ProfitTrait,
    {
        self.oracle.execute(
            FactoryStage4 {
                cmd: self.cmd,
                asset: self.asset,
                lpn: self.lpn,
                lpp: self.lpp,
                profit,
                alarms: self.alarms,
            },
            self.querier,
        )
    }
}

struct FactoryStage4<Cmd, Asset, Lpn, Lpp, Profit> {
    cmd: Cmd,
    asset: PhantomData<Asset>,
    lpn: PhantomData<Lpn>,
    lpp: Lpp,
    profit: Profit,
    alarms: TimeAlarmsRef,
}

impl<Cmd, Asset, Lpn, Lpp, Profit> WithOracle<Lpn> for FactoryStage4<Cmd, Asset, Lpn, Lpp, Profit>
where
    Cmd: WithLeaseDeps,
    Asset: Currency + Serialize,
    Lpn: Currency + Serialize,
    Lpp: LppLenderTrait<Lpn>,
    Profit: ProfitTrait,
{
    type Output = Cmd::Output;
    type Error = Cmd::Error;

    fn exec<Oracle>(self, oracle: Oracle) -> Result<Self::Output, Self::Error>
    where
        Oracle: OracleTrait<Lpn>,
    {
        self.alarms.execute(FactoryStage5 {
            cmd: self.cmd,
            asset: self.asset,
            lpn: self.lpn,
            lpp: self.lpp,
            profit: self.profit,
            oracle,
        })
    }
}

struct FactoryStage5<Cmd, Asset, Lpn, Lpp, Profit, Oracle> {
    cmd: Cmd,
    asset: PhantomData<Asset>,
    lpn: PhantomData<Lpn>,
    lpp: Lpp,
    profit: Profit,
    oracle: Oracle,
}

impl<Cmd, Asset, Lpn, Lpp, Profit, Oracle> WithTimeAlarms
    for FactoryStage5<Cmd, Asset, Lpn, Lpp, Profit, Oracle>
where
    Cmd: WithLeaseDeps,
    Asset: Currency + Serialize,
    Lpn: Currency + Serialize,
    Lpp: LppLenderTrait<Lpn>,
    Profit: ProfitTrait,
    Oracle: OracleTrait<Lpn>,
{
    type Output = Cmd::Output;
    type Error = Cmd::Error;

    fn exec<TimeAlarms>(self, alarms: TimeAlarms) -> Result<Self::Output, Self::Error>
    where
        TimeAlarms: TimeAlarmsTrait,
    {
        self.cmd
            .exec::<_, Asset, _, _, _, _>(self.lpp, self.profit, alarms, self.oracle)
    }
}
