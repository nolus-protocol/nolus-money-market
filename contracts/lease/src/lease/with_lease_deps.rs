use std::marker::PhantomData;

use cosmwasm_std::Addr;
use serde::{de::DeserializeOwned, Serialize};

use ::currency::lease::LeaseGroup;
use finance::currency::{self, AnyVisitor, AnyVisitorResult, Currency, Symbol};
use lpp::stub::{
    loan::{LppLoan as LppLoanTrait, WithLppLoan},
    LppRef,
};
use oracle::stub::{Oracle as OracleTrait, OracleRef, WithOracle};
use profit::stub::{Profit as ProfitTrait, ProfitRef, WithProfit};
use sdk::cosmwasm_std::QuerierWrapper;

pub trait WithLeaseDeps {
    type Output;
    type Error;

    fn exec<Lpn, Asset, LppLoan, Profit, Oracle>(
        self,
        lpp_loan: LppLoan,
        profit: Profit,
        oracle: Oracle,
    ) -> Result<Self::Output, Self::Error>
    where
        Lpn: Currency + Serialize,
        LppLoan: LppLoanTrait<Lpn>,
        Oracle: OracleTrait<Lpn>,
        Profit: ProfitTrait,
        Asset: Currency + Serialize;
}

pub fn execute<Cmd>(
    cmd: Cmd,
    lease_addr: Addr,
    asset: Symbol<'_>,
    lpp: LppRef,
    profit: ProfitRef,
    oracle: OracleRef,
    querier: &QuerierWrapper<'_>,
) -> Result<Cmd::Output, Cmd::Error>
where
    Cmd: WithLeaseDeps,
    Cmd::Error: From<lpp::error::ContractError>,
    finance::error::Error: Into<Cmd::Error>,
    oracle::error::ContractError: Into<Cmd::Error>,
    profit::error::ContractError: Into<Cmd::Error>,
{
    currency::visit_any_on_ticker::<LeaseGroup, _>(
        asset,
        FactoryStage1 {
            cmd,
            lease_addr,
            lpp,
            profit,
            oracle,
            querier,
        },
    )
}

struct FactoryStage1<'r, Cmd> {
    cmd: Cmd,
    lease_addr: Addr,
    lpp: LppRef,
    profit: ProfitRef,
    oracle: OracleRef,
    querier: &'r QuerierWrapper<'r>,
}

impl<'r, Cmd> AnyVisitor for FactoryStage1<'r, Cmd>
where
    Cmd: WithLeaseDeps,
    Cmd::Error: From<lpp::error::ContractError>,
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
        self.lpp.execute_loan(
            FactoryStage2 {
                cmd: self.cmd,
                asset: PhantomData::<C>,
                profit: self.profit,
                oracle: self.oracle,
                querier: self.querier,
            },
            self.lease_addr,
            self.querier,
        )
    }
}
struct FactoryStage2<'r, Cmd, Asset> {
    cmd: Cmd,
    asset: PhantomData<Asset>,
    profit: ProfitRef,
    oracle: OracleRef,
    querier: &'r QuerierWrapper<'r>,
}

impl<'r, Cmd, Asset> WithLppLoan for FactoryStage2<'r, Cmd, Asset>
where
    Cmd: WithLeaseDeps,
    Asset: Currency + Serialize,
    lpp::error::ContractError: Into<Cmd::Error>,
    oracle::error::ContractError: Into<Cmd::Error>,
    profit::error::ContractError: Into<Cmd::Error>,
{
    type Output = Cmd::Output;
    type Error = Cmd::Error;

    fn exec<Lpn, LppLoan>(self, lpp: LppLoan) -> Result<Self::Output, Self::Error>
    where
        Lpn: Currency + Serialize,
        LppLoan: LppLoanTrait<Lpn>,
    {
        self.profit.execute(FactoryStage3 {
            cmd: self.cmd,
            asset: self.asset,
            lpn: PhantomData::<Lpn>,
            lpp_loan: lpp,
            oracle: self.oracle,
            querier: self.querier,
        })
    }
}

struct FactoryStage3<'r, Cmd, Asset, Lpn, LppLoan> {
    cmd: Cmd,
    asset: PhantomData<Asset>,
    lpn: PhantomData<Lpn>,
    lpp_loan: LppLoan,
    oracle: OracleRef,
    querier: &'r QuerierWrapper<'r>,
}

impl<'r, Cmd, Asset, Lpn, LppLoan> WithProfit for FactoryStage3<'r, Cmd, Asset, Lpn, LppLoan>
where
    Cmd: WithLeaseDeps,
    Asset: Currency + Serialize,
    Lpn: Currency + Serialize,
    LppLoan: LppLoanTrait<Lpn>,
    oracle::error::ContractError: Into<Cmd::Error>,
{
    type Output = Cmd::Output;
    type Error = Cmd::Error;

    fn exec<P>(self, profit: P) -> Result<Self::Output, Self::Error>
    where
        P: ProfitTrait,
    {
        self.oracle.execute_as_oracle(
            FactoryStage4 {
                cmd: self.cmd,
                asset: self.asset,
                lpn: self.lpn,
                lpp_loan: self.lpp_loan,
                profit,
            },
            self.querier,
        )
    }
}

struct FactoryStage4<Cmd, Asset, Lpn, LppLoan, Profit> {
    cmd: Cmd,
    asset: PhantomData<Asset>,
    lpn: PhantomData<Lpn>,
    lpp_loan: LppLoan,
    profit: Profit,
}

impl<Cmd, Asset, Lpn, LppLoan, Profit> WithOracle<Lpn>
    for FactoryStage4<Cmd, Asset, Lpn, LppLoan, Profit>
where
    Cmd: WithLeaseDeps,
    Asset: Currency + Serialize,
    Lpn: Currency + Serialize,
    LppLoan: LppLoanTrait<Lpn>,
    Profit: ProfitTrait,
{
    type Output = Cmd::Output;
    type Error = Cmd::Error;

    fn exec<Oracle>(self, oracle: Oracle) -> Result<Self::Output, Self::Error>
    where
        Oracle: OracleTrait<Lpn>,
    {
        self.cmd
            .exec::<_, Asset, _, _, _>(self.lpp_loan, self.profit, oracle)
    }
}
