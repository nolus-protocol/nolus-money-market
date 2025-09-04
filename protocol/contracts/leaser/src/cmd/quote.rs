use std::marker::PhantomData;

use currency::{AnyVisitor, Currency, CurrencyDTO, CurrencyDef, MemberOf};
use finance::{
    coin::{Coin, WithCoin},
    liability::Liability,
    percent::Percent,
    price::total,
};
use lease::api::DownpaymentCoin;
use lpp::{
    msg::QueryQuoteResponse,
    stub::lender::{LppLender as LppLenderTrait, WithLppLender},
};
use oracle_platform::{Oracle as OracleTrait, WithOracle};
use sdk::cosmwasm_std::QuerierWrapper;

use crate::{
    ContractError,
    finance::{LeaseCurrencies, LpnCurrencies, LpnCurrency, OracleRef, PaymentCurrencies},
    msg::QuoteResponse,
    result::ContractResult,
};

pub struct Quote<'r> {
    querier: QuerierWrapper<'r>,
    lease_asset: CurrencyDTO<LeaseCurrencies>,
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
        lease_asset: CurrencyDTO<LeaseCurrencies>,
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

impl WithLppLender<LpnCurrency> for Quote<'_> {
    type Output = QuoteResponse;
    type Error = ContractError;

    fn exec<Lpp>(self, lpp: Lpp) -> Result<Self::Output, Self::Error>
    where
        Lpp: LppLenderTrait<LpnCurrency>,
    {
        self.oracle.execute_as_oracle(
            QuoteStage2 {
                downpayment: self.downpayment,
                lease_asset: self.lease_asset,
                lpp_quote: LppQuote::new(lpp),
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
    Lpp: LppLenderTrait<Lpn>,
{
    pub fn new(lpp: Lpp) -> Self {
        Self {
            lpn: PhantomData,
            lpp,
        }
    }

    pub fn with(&self, borrow: Coin<Lpn>) -> Result<Percent, ContractError> {
        if borrow.is_zero() {
            return Err(ContractError::ZeroDownpayment {});
        }

        self.lpp
            .quote(borrow)
            .map_err(ContractError::QuoteQuery)
            .and_then(|quote_resp| match quote_resp {
                QueryQuoteResponse::QuoteInterestRate(rate) => Ok(rate),
                QueryQuoteResponse::NoLiquidity => Err(ContractError::NoLiquidity {}),
            })
    }
}

struct QuoteStage2<Lpn, Lpp>
where
    Lpp: LppLenderTrait<Lpn>,
{
    downpayment: DownpaymentCoin,
    lease_asset: CurrencyDTO<LeaseCurrencies>,
    lpp_quote: LppQuote<Lpn, Lpp>,
    liability: Liability,
    lease_interest_rate_margin: Percent,
    max_ltd: Option<Percent>,
}

impl<Lpn, Lpp> WithOracle<Lpn, LpnCurrencies> for QuoteStage2<Lpn, Lpp>
where
    Lpn: CurrencyDef,
    Lpn::Group: MemberOf<LpnCurrencies>,
    Lpp: LppLenderTrait<Lpn>,
{
    type G = PaymentCurrencies;
    type Output = QuoteResponse;
    type Error = ContractError;

    fn exec<O>(self, oracle: O) -> Result<Self::Output, Self::Error>
    where
        O: OracleTrait<Self::G, QuoteC = Lpn, QuoteG = LpnCurrencies>,
    {
        self.downpayment.with_coin(QuoteStage3 {
            lease_asset: self.lease_asset,
            lpp_quote: self.lpp_quote,
            oracle,
            liability: self.liability,
            lease_interest_rate_margin: self.lease_interest_rate_margin,
            max_ltd: self.max_ltd,
        })
    }
}

struct QuoteStage3<Lpn, Lpp, Oracle>
where
    Lpp: LppLenderTrait<Lpn>,
    Oracle: OracleTrait<PaymentCurrencies, QuoteC = Lpn, QuoteG = LpnCurrencies>,
{
    lease_asset: CurrencyDTO<LeaseCurrencies>,
    lpp_quote: LppQuote<Lpn, Lpp>,
    oracle: Oracle,
    liability: Liability,
    lease_interest_rate_margin: Percent,
    max_ltd: Option<Percent>,
}

impl<Lpn, Lpp, Oracle> WithCoin<PaymentCurrencies> for QuoteStage3<Lpn, Lpp, Oracle>
where
    Lpn: CurrencyDef,
    Lpn::Group: MemberOf<LpnCurrencies>,
    Lpp: LppLenderTrait<Lpn>,
    Oracle: OracleTrait<PaymentCurrencies, QuoteC = Lpn, QuoteG = LpnCurrencies>,
{
    type Outcome = ContractResult<QuoteResponse>;

    fn on<Dpc>(self, downpayment: Coin<Dpc>) -> Self::Outcome
    where
        Dpc: CurrencyDef,
        Dpc::Group: MemberOf<PaymentCurrencies>,
    {
        self.lease_asset.into_currency_type(QuoteStage4 {
            downpayment,
            lpp_quote: self.lpp_quote,
            oracle: self.oracle,
            liability: self.liability,
            lease_interest_rate_margin: self.lease_interest_rate_margin,
            max_ltd: self.max_ltd,
        })
    }
}

struct QuoteStage4<Lpn, Dpc, Lpp, Oracle>
where
    Dpc: Currency + MemberOf<PaymentCurrencies>,
    Lpp: LppLenderTrait<Lpn>,
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
    Lpn: CurrencyDef,
    Lpn::Group: MemberOf<LpnCurrencies>,
    Dpc: CurrencyDef,
    Dpc::Group: MemberOf<PaymentCurrencies>,
    Lpp: LppLenderTrait<Lpn>,
    Oracle: OracleTrait<PaymentCurrencies, QuoteC = Lpn, QuoteG = LpnCurrencies>,
{
    type Outcome = ContractResult<QuoteResponse>;

    fn on<Asset>(self, _def: &CurrencyDTO<Asset::Group>) -> Self::Outcome
    where
        Asset: CurrencyDef,
        Asset::Group: MemberOf<LeaseCurrencies> + MemberOf<PaymentCurrencies>,
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
