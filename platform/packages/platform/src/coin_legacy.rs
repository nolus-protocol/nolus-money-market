use std::{marker::PhantomData, result::Result as StdResult};

#[cfg(any(test, feature = "testing"))]
use currency::DexSymbols;
use currency::{
    group::MemberOf, AnyVisitor, AnyVisitorResult, BankSymbols, Currency, CurrencyVisit,
    Definition, Group, GroupVisit, SingleVisitor, Symbol,
};
use finance::coin::{Amount, Coin, CoinDTO, WithCoin, WithCoinResult};
use sdk::cosmwasm_std::Coin as CosmWasmCoin;

use crate::{error::Error, result::Result};

pub(crate) fn from_cosmwasm<C>(coin: CosmWasmCoin) -> Result<Coin<C>>
where
    C: Currency + Definition,
{
    from_cosmwasm_currency_not_definition::<C, C>(coin)
}

pub(crate) fn from_cosmwasm_currency_not_definition<COut, CDef>(
    coin: CosmWasmCoin,
) -> Result<Coin<COut>>
where
    COut: 'static,
    CDef: Currency + Definition,
{
    BankSymbols::visit::<CDef, _>(&coin.denom, CoinTransformer(&coin, PhantomData))
}

pub(crate) fn from_cosmwasm_any<G, V>(
    coin: &CosmWasmCoin,
    v: V,
) -> StdResult<WithCoinResult<G, V>, V>
where
    G: Group,
    V: WithCoin<G>,
{
    BankSymbols::maybe_visit_any(&coin.denom, CoinTransformerAny(coin, PhantomData::<G>, v))
        .map_err(|transformer| transformer.2)
}

pub(crate) fn maybe_from_cosmwasm_any<G, V>(
    coin: &CosmWasmCoin,
    v: V,
) -> Option<WithCoinResult<G, V>>
where
    G: Group,
    V: WithCoin<G>,
{
    BankSymbols::maybe_visit_any(&coin.denom, CoinTransformerAny(coin, PhantomData::<G>, v)).ok()
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
    S: Symbol,
{
    struct CoinTransformer<G, S>(PhantomData<G>, PhantomData<S>);
    impl<G, S> WithCoin<G> for CoinTransformer<G, S>
    where
        G: Group,
        S: Symbol,
    {
        type Output = CosmWasmCoin;
        type Error = Error;

        fn on<C>(self, coin: Coin<C>) -> WithCoinResult<G, Self>
        where
            C: Currency,
        {
            Ok(to_cosmwasm_on_network_impl::<C, S>(coin))
        }
    }
    coin_dto.with_coin(CoinTransformer(PhantomData::<G>, PhantomData::<S>))
}

fn to_cosmwasm_on_network_impl<C, S>(coin: Coin<C>) -> CosmWasmCoin
where
    C: Currency,
    S: Symbol,
{
    CosmWasmCoin::new(coin.into(), currency::symbol::<C, S>())
}

struct CoinTransformer<'a, COut>(&'a CosmWasmCoin, PhantomData<COut>)
where
    COut: 'static;

impl<'a, CDef, COut> SingleVisitor<CDef> for CoinTransformer<'a, COut>
where
    CDef: 'static + Definition,
{
    type Output = Coin<COut>;

    type Error = Error;

    fn on(self) -> std::result::Result<Self::Output, Self::Error> {
        Ok(from_cosmwasm_internal::<CDef, _>(self.0))
    }
}

struct CoinTransformerAny<'a, G, V>(&'a CosmWasmCoin, PhantomData<G>, V);

impl<'a, G, V> AnyVisitor for CoinTransformerAny<'a, G, V>
where
    G: Group,
    V: WithCoin<G>,
{
    type VisitedG = G;
    type Output = V::Output;
    type Error = V::Error;

    fn on<C>(self) -> AnyVisitorResult<Self>
    where
        C: 'static + Currency + MemberOf<G> + Definition,
    {
        self.2.on::<C>(from_cosmwasm_internal::<C, _>(self.0))
    }
}

fn from_cosmwasm_internal<CDef, COut>(coin: &CosmWasmCoin) -> Coin<COut>
where
    CDef: 'static + Definition,
    COut: 'static,
{
    debug_assert_eq!(CDef::BANK_SYMBOL, coin.denom);
    debug_assert!(currency::equal::<COut, CDef>());
    Amount::from(coin.amount).into()
}

#[cfg(test)]
mod test {
    use currency::{
        test::{SuperGroup, SuperGroupTestC1, SuperGroupTestC2},
        BankSymbols, Definition, Symbol,
    };
    use finance::test::coin;
    use sdk::cosmwasm_std::Coin as CosmWasmCoin;

    use crate::{coin_legacy::from_cosmwasm, error::Error};

    use super::{to_cosmwasm_impl, Coin};

    #[test]
    fn test_add() {
        let c1 = Coin::<SuperGroupTestC2>::new(10);
        let c2 = Coin::<SuperGroupTestC2>::new(20);
        let c12 = Coin::<SuperGroupTestC2>::new(30);
        assert_eq!(c12, c1 + c2);
    }

    #[test]
    fn from_cosmwasm_impl() {
        let c1 = super::from_cosmwasm::<SuperGroupTestC2>(CosmWasmCoin::new(
            12,
            SuperGroupTestC2::BANK_SYMBOL,
        ));
        assert_eq!(Ok(Coin::<SuperGroupTestC2>::new(12)), c1);
    }
    #[test]
    fn from_cosmwasm_unexpected() {
        let c1 = super::from_cosmwasm::<SuperGroupTestC2>(CosmWasmCoin::new(
            12,
            SuperGroupTestC1::BANK_SYMBOL,
        ));

        assert_eq!(
            c1,
            Err(Error::Currency(currency::error::Error::UnexpectedSymbol(
                SuperGroupTestC1::BANK_SYMBOL.into(),
                BankSymbols::DESCR.into(),
                SuperGroupTestC2::TICKER.into()
            ))),
        );

        let c2 = super::from_cosmwasm::<SuperGroupTestC1>(CosmWasmCoin::new(
            12,
            SuperGroupTestC2::BANK_SYMBOL,
        ));

        assert_eq!(
            c2,
            Err(Error::Currency(currency::error::Error::UnexpectedSymbol(
                SuperGroupTestC2::BANK_SYMBOL.into(),
                BankSymbols::DESCR.into(),
                SuperGroupTestC1::TICKER.into()
            ))),
        );
    }

    #[test]
    fn from_cosmwasm_any_impl() {
        let amount = 42;
        type TheCurrency = SuperGroupTestC1;
        assert_eq!(
            Ok(Ok(true)),
            super::from_cosmwasm_any::<SuperGroup, _>(
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
            super::from_cosmwasm_any::<SuperGroup, _>(
                &CosmWasmCoin::new(amount + 1, TheCurrency::BANK_SYMBOL),
                coin::Expect(Coin::<TheCurrency>::from(amount))
            )
        );
        assert_eq!(
            Ok(Ok(false)),
            super::from_cosmwasm_any::<SuperGroup, _>(
                &CosmWasmCoin::new(amount, TheCurrency::BANK_SYMBOL),
                coin::Expect(Coin::<AnotherCurrency>::from(amount))
            )
        );
        let with_coin = coin::Expect(Coin::<TheCurrency>::from(amount));
        assert_eq!(
            Err(with_coin.clone()),
            super::from_cosmwasm_any::<SuperGroup, _>(
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
        assert_eq!(Ok(c_nls), from_cosmwasm(to_cosmwasm_impl(c_nls)));

        let c_usdc = Coin::<SuperGroupTestC1>::new(u128::MAX);
        assert_eq!(Ok(c_usdc), from_cosmwasm(to_cosmwasm_impl(c_usdc)));
    }
}
