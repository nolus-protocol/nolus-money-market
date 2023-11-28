use std::{marker::PhantomData, result::Result as StdResult};

#[cfg(any(test, feature = "testing"))]
use currency::DexSymbols;
use currency::{
    self, AnyVisitor, AnyVisitorResult, BankSymbols, Currency, CurrencyVisit, Group, GroupVisit,
    SingleVisitor, Symbol, Symbols,
};
use finance::coin::{Amount, Coin, CoinDTO, WithCoin, WithCoinResult};
use sdk::cosmwasm_std::Coin as CosmWasmCoin;

use crate::{error::Error, result::Result};

pub(crate) fn from_cosmwasm_impl<C>(coin: CosmWasmCoin) -> Result<Coin<C>>
where
    C: Currency,
{
    BankSymbols.visit(&coin.denom, CoinTransformer(&coin))
}

pub(crate) fn from_cosmwasm_any<G, V>(coin: CosmWasmCoin, v: V) -> StdResult<WithCoinResult<V>, V>
where
    G: Group,
    V: WithCoin,
{
    BankSymbols
        .maybe_visit_any::<G, _>(&coin.denom, CoinTransformerAny(&coin, v))
        .map_err(|transformer| transformer.1)
}

pub(crate) fn maybe_from_cosmwasm_any<G, V>(coin: CosmWasmCoin, v: V) -> Option<WithCoinResult<V>>
where
    G: Group,
    V: WithCoin,
{
    BankSymbols
        .maybe_visit_any::<G, _>(&coin.denom, CoinTransformerAny(&coin, v))
        .ok()
}

#[cfg(any(test, feature = "testing"))]
pub fn to_cosmwasm<C>(coin: Coin<C>) -> CosmWasmCoin
where
    C: Currency,
{
    to_cosmwasm_impl(coin)
}

#[cfg(any(test, feature = "testing"))]
pub fn to_cosmwasm_on_dex<C>(coin: Coin<C>) -> CosmWasmCoin
where
    C: Currency,
{
    to_cosmwasm_on_network_impl::<C, DexSymbols>(coin)
}

pub(crate) fn to_cosmwasm_impl<C>(coin: Coin<C>) -> CosmWasmCoin
where
    C: Currency,
{
    to_cosmwasm_on_network_impl::<C, BankSymbols>(coin)
}

pub fn to_cosmwasm_on_network<G, S>(coin_dto: &CoinDTO<G>) -> Result<CosmWasmCoin>
where
    G: Group,
    S: Symbols,
{
    struct CoinTransformer<CM>(PhantomData<CM>);
    impl<S> WithCoin for CoinTransformer<S>
    where
        S: Symbols,
    {
        type Output = CosmWasmCoin;
        type Error = Error;

        fn on<C>(&self, coin: Coin<C>) -> WithCoinResult<Self>
        where
            C: Currency,
        {
            Ok(to_cosmwasm_on_network_impl::<C, S>(coin))
        }
    }
    coin_dto.with_coin(CoinTransformer(PhantomData::<S>))
}

fn to_cosmwasm_on_network_impl<C, S>(coin: Coin<C>) -> CosmWasmCoin
where
    C: Currency,
    S: Symbols,
{
    CosmWasmCoin::new(coin.into(), <S::Symbol<C>>::VALUE)
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

struct CoinTransformerAny<'a, V>(&'a CosmWasmCoin, V);

impl<'a, V> AnyVisitor for CoinTransformerAny<'a, V>
where
    V: WithCoin,
{
    type Output = V::Output;
    type Error = V::Error;

    fn on<C>(self) -> AnyVisitorResult<Self>
    where
        C: Currency,
    {
        self.1.on::<C>(from_cosmwasm_internal(self.0))
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
        BankSymbols, Currency,
    };
    use finance::test::coin;
    use sdk::cosmwasm_std::Coin as CosmWasmCoin;

    use crate::{coin_legacy::from_cosmwasm_impl, error::Error};

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
                currency::error::Error::unexpected_symbol::<_, BankSymbols, SuperGroupTestC2>(
                    SuperGroupTestC1::BANK_SYMBOL,
                )
            )),
        );

        let c2 = from_cosmwasm_impl::<SuperGroupTestC1>(CosmWasmCoin::new(
            12,
            SuperGroupTestC2::BANK_SYMBOL,
        ));

        assert_eq!(
            c2,
            Err(Error::Currency(
                currency::error::Error::unexpected_symbol::<_, BankSymbols, SuperGroupTestC1>(
                    SuperGroupTestC2::BANK_SYMBOL,
                )
            )),
        );
    }

    #[test]
    fn from_cosmwasm_any_impl() {
        let amount = 42;
        type TheCurrency = SuperGroupTestC1;
        assert_eq!(
            Ok(Ok(true)),
            super::from_cosmwasm_any::<SuperGroup, _>(
                CosmWasmCoin::new(amount, TheCurrency::BANK_SYMBOL),
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
            super::from_cosmwasm_any::<SuperGroup, _>(
                CosmWasmCoin::new(amount + 1, TheCurrency::BANK_SYMBOL),
                coin::Expect(Coin::<TheCurrency>::from(amount))
            )
        );
        assert_eq!(
            Ok(Ok(false)),
            super::from_cosmwasm_any::<SuperGroup, _>(
                CosmWasmCoin::new(amount, TheCurrency::BANK_SYMBOL),
                coin::Expect(Coin::<AnotherCurrency>::from(amount))
            )
        );
        let with_coin = coin::Expect(Coin::<TheCurrency>::from(amount));
        assert_eq!(
            Err(with_coin.clone()),
            super::from_cosmwasm_any::<SuperGroup, _>(
                CosmWasmCoin::new(amount, TheCurrency::DEX_SYMBOL),
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
}
