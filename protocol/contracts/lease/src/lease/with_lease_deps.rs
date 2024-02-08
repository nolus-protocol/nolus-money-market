use std::marker::PhantomData;

use serde::de::DeserializeOwned;

use currencies::LeaseGroup;
use currency::{
    self, AnyVisitor, AnyVisitorResult, Currency, Group, GroupVisit, SymbolSlice, Tickers,
};
use lpp::stub::{
    loan::{LppLoan as LppLoanTrait, WithLppLoan},
    LppRef,
};
use oracle_platform::{Oracle as OracleTrait, OracleRef, WithOracle};
use sdk::cosmwasm_std::{Addr, QuerierWrapper};

use crate::api::LpnCurrencies;

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
        Asset: Currency,
        LppLoan: LppLoanTrait<Lpn, LpnCurrencies>,
        Oracle: OracleTrait<Lpn>;
}

pub fn execute<Cmd, Lpns>(
    cmd: Cmd,
    lease_addr: Addr,
    asset: &SymbolSlice,
    lpp: LppRef<Lpns>,
    oracle: OracleRef,
    querier: QuerierWrapper<'_>,
) -> Result<Cmd::Output, Cmd::Error>
where
    Cmd: WithLeaseDeps,
    Lpns: Group,
    Cmd::Error: From<lpp::error::ContractError>,
    currency::error::Error: Into<Cmd::Error>,
    oracle_platform::error::Error: Into<Cmd::Error>,
{
    Tickers.visit_any::<LeaseGroup, _>(
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

struct FactoryStage1<'r, Cmd, Lpns> {
    cmd: Cmd,
    lease_addr: Addr,
    lpp: LppRef<Lpns>,
    oracle: OracleRef,
    querier: QuerierWrapper<'r>,
}

impl<'r, Cmd, Lpns> AnyVisitor for FactoryStage1<'r, Cmd, Lpns>
where
    Cmd: WithLeaseDeps,
    Lpns: Group,
    Cmd::Error: From<lpp::error::ContractError>,
    currency::error::Error: Into<Cmd::Error>,
    oracle_platform::error::Error: Into<Cmd::Error>,
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
    querier: QuerierWrapper<'r>,
}

impl<'r, Cmd, Asset> WithLppLoan for FactoryStage2<'r, Cmd, Asset>
where
    Cmd: WithLeaseDeps,
    Asset: Currency,
    lpp::error::ContractError: Into<Cmd::Error>,
    oracle_platform::error::Error: Into<Cmd::Error>,
{
    type Output = Cmd::Output;
    type Error = Cmd::Error;

    fn exec<Lpn, Lpns, LppLoan>(self, lpp_loan: LppLoan) -> Result<Self::Output, Self::Error>
    where
        Lpn: Currency,
        Lpns: Group,
        LppLoan: LppLoanTrait<Lpn, Lpns>,
    {
        self.oracle.execute_as_oracle::<Lpn, Lpns, _>(
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
    LppLoan: LppLoanTrait<Lpn, LpnCurrencies>,
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
