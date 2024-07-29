use currency::{self, Currency, MemberOf};
use lpp::stub::loan::{LppLoan as LppLoanTrait, WithLppLoan};
use oracle_platform::{Oracle as OracleTrait, WithOracle};
use sdk::cosmwasm_std::{Addr, QuerierWrapper};

use crate::{
    api::{LeaseAssetCurrencies, LeasePaymentCurrencies},
    error::ContractError,
    finance::{LpnCurrencies, LpnCurrency, LppRef, OracleRef},
    position::{Position, PositionDTO, WithPosition, WithPositionResult},
};

pub trait WithLeaseDeps {
    type Output;
    type Error;

    fn exec<Lpn, Asset, LppLoan, Oracle>(
        self,
        position: Position<Asset>,
        lpp_loan: LppLoan,
        oracle: Oracle,
    ) -> Result<Self::Output, Self::Error>
    where
        Asset: Currency + MemberOf<LeaseAssetCurrencies> + MemberOf<LeasePaymentCurrencies>,
        LppLoan: LppLoanTrait<LpnCurrency, LpnCurrencies>,
        Oracle: OracleTrait<LeasePaymentCurrencies, QuoteC = LpnCurrency, QuoteG = LpnCurrencies>
            + Into<OracleRef>;
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
    Cmd::Error: From<lpp::error::ContractError> + From<finance::error::Error> + From<ContractError>,
    // currency::error::Error: Into<Cmd::Error>,
    oracle_platform::error::Error: Into<Cmd::Error>,
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

impl<'r, Cmd> WithPosition for FactoryStage1<'r, Cmd>
where
    Cmd: WithLeaseDeps,
    Cmd::Error: From<lpp::error::ContractError>,
    // currency::error::Error: Into<Cmd::Error>,
    oracle_platform::error::Error: Into<Cmd::Error>,
{
    type Output = Cmd::Output;
    type Error = Cmd::Error;

    fn on<Asset>(self, position: Position<Asset>) -> WithPositionResult<Self>
    where
        Asset: Currency + MemberOf<LeaseAssetCurrencies> + MemberOf<LeasePaymentCurrencies>,
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

impl<'r, Cmd, Asset> WithLppLoan<LpnCurrency, LpnCurrencies> for FactoryStage2<'r, Cmd, Asset>
where
    Cmd: WithLeaseDeps,
    Asset: Currency + MemberOf<LeaseAssetCurrencies> + MemberOf<LeasePaymentCurrencies>,
    lpp::error::ContractError: Into<Cmd::Error>,
    oracle_platform::error::Error: Into<Cmd::Error>,
{
    type Output = Cmd::Output;
    type Error = Cmd::Error;

    fn exec<LppLoan>(self, lpp_loan: LppLoan) -> Result<Self::Output, Self::Error>
    where
        LppLoan: LppLoanTrait<LpnCurrency, LpnCurrencies>,
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
    Asset: Currency + MemberOf<LeaseAssetCurrencies> + MemberOf<LeasePaymentCurrencies>,
    LppLoan: LppLoanTrait<LpnCurrency, LpnCurrencies>,
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
