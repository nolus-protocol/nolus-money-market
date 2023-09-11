use std::marker::PhantomData;

use serde::de::DeserializeOwned;

use currency::{
    self, lease::LeaseGroup, AnyVisitor, AnyVisitorResult, Currency, GroupVisit, SymbolSlice,
    TickerMatcher,
};
use lpp::stub::{
    loan::{LppLoan as LppLoanTrait, WithLppLoan},
    LppRef,
};
use oracle::stub::{Oracle as OracleTrait, OracleRef, WithOracle};
use sdk::cosmwasm_std::{Addr, QuerierWrapper};

pub trait WithLeaseDeps {
    type Output;
    type Error;

    fn exec<Lpn, Asset, LppLoan, Oracle>(
        self,
        lpp_loan: LppLoan,
        oracle: Oracle,
    ) -> Result<Self::Output, Self::Error>
    where
        Lpn: Currency,
        LppLoan: LppLoanTrait<Lpn>,
        Oracle: OracleTrait<Lpn>,
        Asset: Currency;
}

pub fn execute<Cmd>(
    cmd: Cmd,
    lease_addr: Addr,
    asset: &SymbolSlice,
    lpp: LppRef,
    oracle: OracleRef,
    querier: &QuerierWrapper<'_>,
) -> Result<Cmd::Output, Cmd::Error>
where
    Cmd: WithLeaseDeps,
    Cmd::Error: From<lpp::error::ContractError>,
    currency::error::Error: Into<Cmd::Error>,
    oracle::error::ContractError: Into<Cmd::Error>,
{
    TickerMatcher.visit_any::<LeaseGroup, _>(
        asset,
        FactoryStage1 {
            cmd,
            lease_addr,
            lpp,
            oracle,
            querier,
        },
    )
}

struct FactoryStage1<'r, Cmd> {
    cmd: Cmd,
    lease_addr: Addr,
    lpp: LppRef,
    oracle: OracleRef,
    querier: &'r QuerierWrapper<'r>,
}

impl<'r, Cmd> AnyVisitor for FactoryStage1<'r, Cmd>
where
    Cmd: WithLeaseDeps,
    Cmd::Error: From<lpp::error::ContractError>,
    currency::error::Error: Into<Cmd::Error>,
    oracle::error::ContractError: Into<Cmd::Error>,
{
    type Output = Cmd::Output;
    type Error = Cmd::Error;

    fn on<C>(self) -> AnyVisitorResult<Self>
    where
        C: 'static + Currency + DeserializeOwned,
    {
        self.lpp.execute_loan(
            FactoryStage2 {
                cmd: self.cmd,
                asset: PhantomData::<C>,
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
    oracle: OracleRef,
    querier: &'r QuerierWrapper<'r>,
}

impl<'r, Cmd, Asset> WithLppLoan for FactoryStage2<'r, Cmd, Asset>
where
    Cmd: WithLeaseDeps,
    Asset: Currency,
    lpp::error::ContractError: Into<Cmd::Error>,
    oracle::error::ContractError: Into<Cmd::Error>,
{
    type Output = Cmd::Output;
    type Error = Cmd::Error;

    fn exec<Lpn, LppLoan>(self, lpp_loan: LppLoan) -> Result<Self::Output, Self::Error>
    where
        Lpn: Currency,
        LppLoan: LppLoanTrait<Lpn>,
    {
        self.oracle.execute_as_oracle(
            FactoryStage4 {
                cmd: self.cmd,
                asset: self.asset,
                lpn: PhantomData::<Lpn>,
                lpp_loan,
            },
            self.querier,
        )
    }
}

struct FactoryStage4<Cmd, Asset, Lpn, LppLoan> {
    cmd: Cmd,
    asset: PhantomData<Asset>,
    lpn: PhantomData<Lpn>,
    lpp_loan: LppLoan,
}

impl<Cmd, Asset, Lpn, LppLoan> WithOracle<Lpn> for FactoryStage4<Cmd, Asset, Lpn, LppLoan>
where
    Cmd: WithLeaseDeps,
    Asset: Currency,
    Lpn: Currency,
    LppLoan: LppLoanTrait<Lpn>,
{
    type Output = Cmd::Output;
    type Error = Cmd::Error;

    fn exec<Oracle>(self, oracle: Oracle) -> Result<Self::Output, Self::Error>
    where
        Oracle: OracleTrait<Lpn>,
    {
        self.cmd.exec::<_, Asset, _, _>(self.lpp_loan, oracle)
    }
}
