use currency::{self, CurrencyDef, MemberOf};
use lpp::stub::loan::{LppLoan as LppLoanTrait, WithLppLoan};
use oracle_platform::{Oracle as OracleTrait, WithOracle};
use sdk::cosmwasm_std::{Addr, QuerierWrapper};

use crate::{
    api::{LeaseAssetCurrencies, LeasePaymentCurrencies},
    finance::{LpnCurrencies, LpnCurrency, LppRef, OracleRef},
    position::{Position, PositionDTO, PositionError, WithPosition, WithPositionResult},
};

pub(crate) trait WithLeaseDeps {
    type Output;
    type Error;

    fn exec<Lpn, Asset, LppLoan, Oracle>(
        self,
        position: Position<Asset>,
        lpp_loan: LppLoan,
        oracle: Oracle,
    ) -> Result<Self::Output, Self::Error>
    where
        Asset: CurrencyDef,
        Asset::Group: MemberOf<LeaseAssetCurrencies> + MemberOf<LeasePaymentCurrencies>,
        LppLoan: LppLoanTrait<LpnCurrency>,
        Oracle: OracleTrait<LeasePaymentCurrencies, QuoteC = LpnCurrency, QuoteG = LpnCurrencies>
            + Into<OracleRef>;
}

pub fn execute_resolved_position<Cmd, Asset>(
    cmd: Cmd,
    lease_addr: Addr,
    position: Position<Asset>,
    lpp: LppRef,
    oracle: OracleRef,
    querier: QuerierWrapper<'_>,
) -> Result<Cmd::Output, Cmd::Error>
where
    Cmd: WithLeaseDeps,
    Cmd::Error: From<lpp::stub::lender::Error> + From<finance::error::Error> + From<PositionError>,
    Asset: CurrencyDef,
    Asset::Group: MemberOf<LeaseAssetCurrencies> + MemberOf<LeasePaymentCurrencies>,
{
    FactoryStage1 {
        cmd,
        lease_addr,
        lpp,
        oracle,
        querier,
    }
    .on(position)
}

pub fn execute<Cmd>(
    cmd: Cmd,
    lease_addr: Addr,
    position: PositionDTO,
    lpp: LppRef,
    oracle: OracleRef,
    querier: QuerierWrapper<'_>,
) -> Result<Cmd::Output, Cmd::Error>
where
    Cmd: WithLeaseDeps,
    PositionError: Into<Cmd::Error>,
    lpp::stub::lender::Error: Into<Cmd::Error>,
{
    position.with_position(FactoryStage1 {
        cmd,
        lease_addr,
        lpp,
        oracle,
        querier,
    })
}

struct FactoryStage1<'r, Cmd> {
    cmd: Cmd,
    lease_addr: Addr,
    lpp: LppRef,
    oracle: OracleRef,
    querier: QuerierWrapper<'r>,
}

impl<Cmd> WithPosition for FactoryStage1<'_, Cmd>
where
    Cmd: WithLeaseDeps,
    lpp::stub::lender::Error: Into<Cmd::Error>,
{
    type Output = Cmd::Output;
    type Error = Cmd::Error;

    fn on<Asset>(self, position: Position<Asset>) -> WithPositionResult<Self>
    where
        Asset: CurrencyDef,
        Asset::Group: MemberOf<LeaseAssetCurrencies> + MemberOf<LeasePaymentCurrencies>,
    {
        self.lpp.execute_loan(
            FactoryStage2 {
                cmd: self.cmd,
                position,
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
    position: Position<Asset>,
    oracle: OracleRef,
    querier: QuerierWrapper<'r>,
}

impl<Cmd, Asset> WithLppLoan<LpnCurrency> for FactoryStage2<'_, Cmd, Asset>
where
    Cmd: WithLeaseDeps,
    Asset: CurrencyDef,
    Asset::Group: MemberOf<LeaseAssetCurrencies> + MemberOf<LeasePaymentCurrencies>,
    lpp::stub::lender::Error: Into<Cmd::Error>,
{
    type Output = Cmd::Output;
    type Error = Cmd::Error;

    fn exec<LppLoan>(self, lpp_loan: LppLoan) -> Result<Self::Output, Self::Error>
    where
        LppLoan: LppLoanTrait<LpnCurrency>,
    {
        self.oracle.execute_as_oracle(
            FactoryStage4 {
                cmd: self.cmd,
                position: self.position,
                lpp_loan,
            },
            self.querier,
        )
    }
}

struct FactoryStage4<Cmd, Asset, LppLoan> {
    cmd: Cmd,
    position: Position<Asset>,
    lpp_loan: LppLoan,
}

impl<Cmd, Asset, LppLoan> WithOracle<LpnCurrency, LpnCurrencies>
    for FactoryStage4<Cmd, Asset, LppLoan>
where
    Cmd: WithLeaseDeps,
    Asset: CurrencyDef,
    Asset::Group: MemberOf<LeaseAssetCurrencies> + MemberOf<LeasePaymentCurrencies>,
    LppLoan: LppLoanTrait<LpnCurrency>,
{
    type G = LeasePaymentCurrencies;

    type Output = Cmd::Output;
    type Error = Cmd::Error;

    fn exec<Oracle>(self, oracle: Oracle) -> Result<Self::Output, Self::Error>
    where
        Oracle:
            OracleTrait<Self::G, QuoteC = LpnCurrency, QuoteG = LpnCurrencies> + Into<OracleRef>,
    {
        self.cmd
            .exec::<LpnCurrency, Asset, _, _>(self.position, self.lpp_loan, oracle)
    }
}
