use std::{marker::PhantomData, result::Result as StdResult};

use currency::{
    AnyVisitor, AnyVisitorResult, BankSymbols, Currency, CurrencyVisit, DexSymbols, Group,
    GroupVisit, MemberOf, SingleVisitor, Symbol, SymbolSlice, SymbolStatic, Tickers,
};
use finance::coin::{Amount, Coin, CoinDTO, WithCoin, WithCoinResult};
use sdk::cosmwasm_std::Coin as CosmWasmCoin;

use crate::{error::Error, result::Result};

pub(crate) fn from_cosmwasm_impl<C>(coin: CosmWasmCoin) -> Result<Coin<C>>
where
    C: Currency,
{
    BankSymbols::visit(&coin.denom, CoinTransformer(&coin))
}

pub(crate) fn from_cosmwasm_any<VisitedG, V>(
    coin: &CosmWasmCoin,
    v: V,
) -> StdResult<WithCoinResult<VisitedG, V>, V>
where
    VisitedG: Group,
    V: WithCoin<VisitedG, VisitorG = VisitedG>,
{
    BankSymbols::maybe_visit_any(
        &coin.denom,
        CoinTransformerAny(coin, PhantomData::<VisitedG>, v),
    )
    .map_err(|transformer| transformer.2)
}

pub(crate) fn maybe_from_cosmwasm_any<VisitedG, V>(
    coin: CosmWasmCoin,
    v: V,
) -> Option<WithCoinResult<VisitedG, V>>
where
    VisitedG: Group,
    V: WithCoin<VisitedG, VisitorG = VisitedG>,
{
    BankSymbols::maybe_visit_any(
        &coin.denom,
        CoinTransformerAny(&coin, PhantomData::<VisitedG>, v),
    )
    .ok()
}

#[cfg(any(test, feature = "testing"))]
pub fn to_cosmwasm<C>(coin: Coin<C>) -> CosmWasmCoin
where
    C: Currency,
{
    to_cosmwasm_impl(coin)
}

pub fn to_cosmwasm_on_dex_symbol<G>(ticker: &SymbolSlice) -> Result<SymbolStatic>
where
    G: Group,
{
    Tickers::visit_any(ticker, DexSymbols::<G>::new()).map_err(Error::from)
}

#[cfg(any(test, feature = "testing"))]
pub fn to_cosmwasm_on_dex<C>(coin: Coin<C>) -> CosmWasmCoin
where
    C: Currency,
{
    to_cosmwasm_on_network_impl::<C, DexSymbols<C::Group>>(coin)
}

pub(crate) fn to_cosmwasm_impl<C>(coin: Coin<C>) -> CosmWasmCoin
where
    C: Currency,
{
    to_cosmwasm_on_network_impl::<C, BankSymbols<C::Group>>(coin)
}

pub fn to_cosmwasm_on_network<S>(coin_dto: &CoinDTO<S::Group>) -> Result<CosmWasmCoin>
where
    S: Symbol,
{
    struct CoinTransformer<CM>(PhantomData<CM>);
    impl<S> WithCoin<S::Group> for CoinTransformer<S>
    where
        S: Symbol,
    {
        type VisitorG = S::Group;
        type Output = CosmWasmCoin;
        type Error = Error;

        fn on<C>(self, coin: Coin<C>) -> WithCoinResult<S::Group, Self>
        where
            C: Currency + MemberOf<Self::VisitorG>,
        {
            Ok(to_cosmwasm_on_network_impl::<C, S>(coin))
        }
    }
    coin_dto.with_coin(CoinTransformer(PhantomData::<S>))
}

fn to_cosmwasm_on_network_impl<C, S>(coin: Coin<C>) -> CosmWasmCoin
where
    C: Currency + MemberOf<S::Group>,
    S: Symbol,
{
    CosmWasmCoin::new(coin.into(), S::symbol::<C>())
}

struct CoinTransformer<'a>(&'a CosmWasmCoin);

impl<'a, C> SingleVisitor<C> for CoinTransformer<'a>
where
    C: Currency,
{
    type Output = Coin<C>;

    type Error = Error;

    fn on(self) -> Result<Self::Output> {
        Ok(from_cosmwasm_internal(self.0))
    }
}

struct CoinTransformerAny<'a, VisitedG, V>(&'a CosmWasmCoin, PhantomData<VisitedG>, V);

impl<'a, VisitedG, V> AnyVisitor<VisitedG> for CoinTransformerAny<'a, VisitedG, V>
where
    VisitedG: Group + MemberOf<V::VisitorG>,
    V: WithCoin<VisitedG>,
{
    type VisitorG = V::VisitorG;
    type Output = V::Output;
    type Error = V::Error;

    fn on<C>(self) -> AnyVisitorResult<VisitedG, Self>
    where
        C: Currency + MemberOf<VisitedG> + MemberOf<Self::VisitorG>,
    {
        self.2.on::<C>(from_cosmwasm_internal(self.0))
    }
}

fn from_cosmwasm_internal<C>(coin: &CosmWasmCoin) -> Coin<C>
where
    C: Currency,
{
    debug_assert_eq!(C::BANK_SYMBOL, coin.denom);
    Amount::from(coin.amount).into()
}

#[cfg(test)]
mod test {
    use currency::{
        test::{SuperGroup, SuperGroupTestC1, SuperGroupTestC2},
        BankSymbols, Currency, SymbolStatic,
    };
    use finance::test::coin;
    use sdk::cosmwasm_std::Coin as CosmWasmCoin;

    use crate::{
        coin_legacy::{from_cosmwasm_impl, to_cosmwasm_on_dex_symbol},
        error::Error,
    };

    use super::{to_cosmwasm_impl, Coin};

    #[test]
    fn test_add() {
        let c1 = Coin::<SuperGroupTestC2>::new(10);
        let c2 = Coin::<SuperGroupTestC2>::new(20);
        let c12 = Coin::<SuperGroupTestC2>::new(30);
        assert_eq!(c12, c1 + c2);
    }

    #[test]
    fn from_cosmwasm() {
        let c1 = from_cosmwasm_impl::<SuperGroupTestC2>(CosmWasmCoin::new(
            12,
            SuperGroupTestC2::BANK_SYMBOL,
        ));
        assert_eq!(Ok(Coin::<SuperGroupTestC2>::new(12)), c1);
    }
    #[test]
    fn from_cosmwasm_unexpected() {
        let c1 = from_cosmwasm_impl::<SuperGroupTestC2>(CosmWasmCoin::new(
            12,
            SuperGroupTestC1::BANK_SYMBOL,
        ));

        assert_eq!(
            c1,
            Err(Error::Currency(
                currency::error::Error::unexpected_symbol::<
                    _,
                    BankSymbols::<SuperGroup>,
                    SuperGroupTestC2,
                >(SuperGroupTestC1::BANK_SYMBOL,)
            )),
        );

        let c2 = from_cosmwasm_impl::<SuperGroupTestC1>(CosmWasmCoin::new(
            12,
            SuperGroupTestC2::BANK_SYMBOL,
        ));

        assert_eq!(
            c2,
            Err(Error::Currency(
                currency::error::Error::unexpected_symbol::<
                    _,
                    BankSymbols::<SuperGroup>,
                    SuperGroupTestC1,
                >(SuperGroupTestC2::BANK_SYMBOL,)
            )),
        );
    }

    #[test]
    fn from_cosmwasm_any_impl() {
        let amount = 42;
        type TheCurrency = SuperGroupTestC1;
        assert_eq!(
            Ok(Ok(true)),
            super::from_cosmwasm_any(
                &CosmWasmCoin::new(amount, TheCurrency::BANK_SYMBOL),
                coin::Expect(Coin::<TheCurrency>::from(amount))
            )
        );
    }

    #[test]
    fn from_cosmwasm_any_impl_err() {
        let amount = 42;
        type TheCurrency = SuperGroupTestC1;
        type AnotherCurrency = SuperGroupTestC2;
        assert_eq!(
            Ok(Ok(false)),
            super::from_cosmwasm_any(
                &CosmWasmCoin::new(amount + 1, TheCurrency::BANK_SYMBOL),
                coin::Expect(Coin::<TheCurrency>::from(amount))
            )
        );
        assert_eq!(
            Ok(Ok(false)),
            super::from_cosmwasm_any(
                &CosmWasmCoin::new(amount, TheCurrency::BANK_SYMBOL),
                coin::Expect(Coin::<AnotherCurrency>::from(amount))
            )
        );
        let with_coin = coin::Expect(Coin::<TheCurrency>::from(amount));
        assert_eq!(
            Err(with_coin.clone()),
            super::from_cosmwasm_any(
                &CosmWasmCoin::new(amount, TheCurrency::DEX_SYMBOL),
                with_coin
            )
        );
    }

    #[test]
    fn to_cosmwasm() {
        let amount = 326;
        assert_eq!(
            CosmWasmCoin::new(amount, SuperGroupTestC2::BANK_SYMBOL),
            to_cosmwasm_impl(Coin::<SuperGroupTestC2>::new(amount))
        );
        assert_eq!(
            CosmWasmCoin::new(amount, SuperGroupTestC1::BANK_SYMBOL),
            to_cosmwasm_impl(Coin::<SuperGroupTestC1>::new(amount))
        );
    }

    #[test]
    fn from_to_cosmwasm() {
        let c_nls = Coin::<SuperGroupTestC2>::new(24563);
        assert_eq!(Ok(c_nls), from_cosmwasm_impl(to_cosmwasm_impl(c_nls)));

        let c_usdc = Coin::<SuperGroupTestC1>::new(u128::MAX);
        assert_eq!(Ok(c_usdc), from_cosmwasm_impl(to_cosmwasm_impl(c_usdc)));
    }

    #[test]
    fn to_dex_symbol() {
        type Currency = SuperGroupTestC1;
        assert_eq!(
            Ok(Currency::DEX_SYMBOL),
            to_cosmwasm_on_dex_symbol::<SuperGroup>(Currency::TICKER)
        );
    }

    #[test]
    fn to_dex_symbol_err() {
        const INVALID_TICKER: SymbolStatic = "NotATicker";
        assert!(matches!(
            to_cosmwasm_on_dex_symbol::<SuperGroup>(INVALID_TICKER),
            Err(Error::Currency(_))
        ));
    }
}
