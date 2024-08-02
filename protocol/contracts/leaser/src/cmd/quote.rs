use std::marker::PhantomData;

use currency::{
    AnyVisitor, AnyVisitorResult, Currency, GroupVisit, MemberOf, SymbolOwned, Tickers,
};
use finance::{coin::Coin, liability::Liability, percent::Percent, price::total};
use lease::api::DownpaymentCoin;
use lpp::{
    msg::QueryQuoteResponse,
    stub::lender::{LppLender as LppLenderTrait, WithLppLender},
};
use oracle_platform::{Oracle as OracleTrait, WithOracle};
use sdk::cosmwasm_std::{QuerierWrapper, StdResult};

use crate::{
    finance::{LeaseCurrencies, LpnCurrencies, LpnCurrency, OracleRef, PaymentCurrencies},
    msg::QuoteResponse,
    ContractError,
};

pub struct Quote<'r> {
    querier: QuerierWrapper<'r>,
    lease_asset: SymbolOwned,
    downpayment: DownpaymentCoin,
    oracle: OracleRef,
    liability: Liability,
    lease_interest_rate_margin: Percent,
    max_ltd: Option<Percent>,
}

impl<'r> Quote<'r> {
    pub fn new(
        querier: QuerierWrapper<'r>,
        downpayment: DownpaymentCoin,
        lease_asset: SymbolOwned,
        oracle: OracleRef,
        liability: Liability,
        lease_interest_rate_margin: Percent,
        max_ltd: Option<Percent>,
    ) -> Self {
        Self {
            querier,
            lease_asset,
            downpayment,
            oracle,
            liability,
            lease_interest_rate_margin,
            max_ltd,
        }
    }
}

impl<'r> WithLppLender<LpnCurrency, LpnCurrencies> for Quote<'r> {
    type Output = QuoteResponse;
    type Error = ContractError;

    fn exec<Lpp>(self, lpp: Lpp) -> Result<Self::Output, Self::Error>
    where
        Lpp: LppLenderTrait<LpnCurrency, LpnCurrencies>,
    {
        self.oracle.execute_as_oracle(
            QuoteStage2 {
                downpayment: self.downpayment,
                lease_asset: self.lease_asset,
                lpp_quote: LppQuote::new(lpp)?,
                liability: self.liability,
                lease_interest_rate_margin: self.lease_interest_rate_margin,
                max_ltd: self.max_ltd,
            },
            self.querier,
        )
    }
}

pub struct LppQuote<Lpn, Lpp> {
    lpn: PhantomData<Lpn>,
    lpp: Lpp,
}

impl<Lpn, Lpp> LppQuote<Lpn, Lpp>
where
    Lpp: LppLenderTrait<Lpn, LpnCurrencies>,
{
    pub fn new(lpp: Lpp) -> StdResult<LppQuote<Lpn, Lpp>> {
        Ok(Self {
            lpn: PhantomData,
            lpp,
        })
    }

    pub fn with(&self, borrow: Coin<Lpn>) -> Result<Percent, ContractError> {
        if borrow.is_zero() {
            return Err(ContractError::ZeroDownpayment {});
        }

        self.lpp
            .quote(borrow)
            .map_err(Into::into)
            .and_then(|quote_resp| match quote_resp {
                QueryQuoteResponse::QuoteInterestRate(rate) => Ok(rate),
                QueryQuoteResponse::NoLiquidity => Err(ContractError::NoLiquidity {}),
            })
    }
}

struct QuoteStage2<Lpn, Lpp>
where
    Lpp: LppLenderTrait<Lpn, LpnCurrencies>,
{
    downpayment: DownpaymentCoin,
    lease_asset: SymbolOwned,
    lpp_quote: LppQuote<Lpn, Lpp>,
    liability: Liability,
    lease_interest_rate_margin: Percent,
    max_ltd: Option<Percent>,
}

impl<Lpn, Lpp> WithOracle<Lpn, LpnCurrencies> for QuoteStage2<Lpn, Lpp>
where
    Lpn: Currency + MemberOf<LpnCurrencies>,
    Lpp: LppLenderTrait<Lpn, LpnCurrencies>,
{
    type G = PaymentCurrencies;
    type Output = QuoteResponse;
    type Error = ContractError;

    fn exec<O>(self, oracle: O) -> Result<Self::Output, Self::Error>
    where
        O: OracleTrait<Self::G, QuoteC = Lpn, QuoteG = LpnCurrencies>,
    {
        // TODO use CoinDTO::with_coin instead
        self.downpayment
            .currency()
            .into_currency_type(QuoteStage3 {
                downpayment: self.downpayment,
                lease_asset: self.lease_asset,
                lpp_quote: self.lpp_quote,
                oracle,
                liability: self.liability,
                lease_interest_rate_margin: self.lease_interest_rate_margin,
                max_ltd: self.max_ltd,
            })
            .map_err(|_| ContractError::UnknownCurrency {
                symbol: self.downpayment.currency().to_string(),
            })
    }
}

struct QuoteStage3<Lpn, Lpp, Oracle>
where
    Lpp: LppLenderTrait<Lpn, LpnCurrencies>,
    Oracle: OracleTrait<PaymentCurrencies, QuoteC = Lpn, QuoteG = LpnCurrencies>,
{
    downpayment: DownpaymentCoin,
    lease_asset: SymbolOwned,
    lpp_quote: LppQuote<Lpn, Lpp>,
    oracle: Oracle,
    liability: Liability,
    lease_interest_rate_margin: Percent,
    max_ltd: Option<Percent>,
}

impl<Lpn, Lpp, Oracle> AnyVisitor<PaymentCurrencies> for QuoteStage3<Lpn, Lpp, Oracle>
where
    Lpn: Currency + MemberOf<LpnCurrencies>,
    Lpp: LppLenderTrait<Lpn, LpnCurrencies>,
    Oracle: OracleTrait<PaymentCurrencies, QuoteC = Lpn, QuoteG = LpnCurrencies>,
{
    type VisitorG = PaymentCurrencies;

    type Output = QuoteResponse;
    type Error = ContractError;

    fn on<C>(self) -> AnyVisitorResult<PaymentCurrencies, Self>
    where
        C: Currency + MemberOf<Self::VisitorG>,
    {
        Tickers::maybe_visit_member_any(
            &self.lease_asset,
            QuoteStage4 {
                downpayment: TryInto::<Coin<C>>::try_into(self.downpayment)?,
                lpp_quote: self.lpp_quote,
                oracle: self.oracle,
                liability: self.liability,
                lease_interest_rate_margin: self.lease_interest_rate_margin,
                max_ltd: self.max_ltd,
            },
        )
        .map_err({
            let symbol = self.lease_asset;

            |_| ContractError::UnknownCurrency { symbol }
        })?
    }
}

struct QuoteStage4<Lpn, Dpc, Lpp, Oracle>
where
    Dpc: Currency + MemberOf<PaymentCurrencies>,
    Lpp: LppLenderTrait<Lpn, LpnCurrencies>,
    Oracle: OracleTrait<PaymentCurrencies, QuoteC = Lpn, QuoteG = LpnCurrencies>,
{
    downpayment: Coin<Dpc>,
    lpp_quote: LppQuote<Lpn, Lpp>,
    oracle: Oracle,
    liability: Liability,
    lease_interest_rate_margin: Percent,
    max_ltd: Option<Percent>,
}

impl<Lpn, Dpc, Lpp, Oracle> AnyVisitor<LeaseCurrencies> for QuoteStage4<Lpn, Dpc, Lpp, Oracle>
where
    Lpn: Currency + MemberOf<LpnCurrencies>,
    Dpc: Currency + MemberOf<PaymentCurrencies>,
    Lpp: LppLenderTrait<Lpn, LpnCurrencies>,
    Oracle: OracleTrait<PaymentCurrencies, QuoteC = Lpn, QuoteG = LpnCurrencies>,
{
    type VisitorG = PaymentCurrencies;

    type Output = QuoteResponse;
    type Error = ContractError;

    fn on<Asset>(self) -> AnyVisitorResult<LeaseCurrencies, Self>
    where
        Asset: Currency + MemberOf<LeaseCurrencies> + MemberOf<Self::VisitorG>,
    {
        let downpayment_lpn = total(self.downpayment, self.oracle.price_of::<Dpc>()?);

        if downpayment_lpn.is_zero() {
            return Err(ContractError::ZeroDownpayment {});
        }

        let borrow = self
            .liability
            .init_borrow_amount(downpayment_lpn, self.max_ltd);

        let asset_price = self.oracle.price_of::<Asset>()?.inv();

        let total_asset = total(downpayment_lpn + borrow, asset_price);

        let annual_interest_rate = self.lpp_quote.with(borrow)?;

        Ok(QuoteResponse {
            total: total_asset.into(),
            borrow: borrow.into(),
            annual_interest_rate,
            annual_interest_rate_margin: self.lease_interest_rate_margin,
        })
    }
}
