use crate::{
    coin::{Coin, Currency, Nls, Usdc},
    error::{Error, Result},
};
use cosmwasm_std::{Coin as CosmWasmCoin, Uint128};

#[deprecated = "Migrate to using finance::coin::Coin"]
pub fn sub_amount(from: CosmWasmCoin, amount: Uint128) -> CosmWasmCoin {
    CosmWasmCoin {
        amount: from.amount - amount,
        denom: from.denom,
    }
}

#[deprecated = "Migrate to using finance::coin::Coin"]
pub fn add_coin(to: CosmWasmCoin, other: CosmWasmCoin) -> CosmWasmCoin {
    debug_assert!(to.denom == other.denom);
    CosmWasmCoin {
        amount: to.amount + other.amount,
        denom: to.denom,
    }
}

pub trait AnyCurrencyVisitor {
    fn on<C>(&mut self, coin: Coin<C>)
    where
        C: Currency;
    fn on_unknown(&mut self);
}

pub trait SingleCurrencyVisitor<C> {
    fn on(&mut self, coin: Coin<C>);
    fn on_unknown(&mut self);
}

pub fn visit_any<V>(coin: &CosmWasmCoin, visitor: &mut V)
where
    V: AnyCurrencyVisitor,
{
    let mut any_visitor = AnyCyrrencyVisitorImpl::new(visitor);
    debug_assert!(!any_visitor.visited());
    visit::<Nls, _>(coin, &mut any_visitor);
    if !any_visitor.visited() {
        visit::<Usdc, _>(coin, &mut any_visitor);
    }
    if !any_visitor.visited() {
        visitor.on_unknown();
    }
}

pub fn visit<C, V>(coin: &CosmWasmCoin, visitor: &mut V)
where
    V: SingleCurrencyVisitor<C>,
    C: Currency,
{
    let amount: u128 = coin.amount.into();
    let currency_symbol = coin.denom.as_str();
    if currency_symbol == C::SYMBOL {
        visitor.on(Coin::<C>::new(amount));
    } else {
        visitor.on_unknown();
    }
}

#[deprecated = "Migrate to using finance::bank::BankAccount"]
pub fn from_cosmwasm<C>(coin: CosmWasmCoin) -> Result<Coin<C>>
where
    C: Currency,
{
    let mut v = CoinTransformer(None);
    visit(&coin, &mut v);
    v.0.ok_or_else(|| Error::UnexpectedCurrency(coin.denom, C::SYMBOL.into()))
}

#[deprecated = "Migrate to using finance::bank::BankAccount"]
pub fn to_cosmwasm<C>(coin: Coin<C>) -> CosmWasmCoin
where
    C: Currency,
{
    CosmWasmCoin::new(coin.amount(), C::SYMBOL)
}

struct AnyCyrrencyVisitorImpl<'a, V>(&'a mut V, bool);
impl<'a, V> AnyCyrrencyVisitorImpl<'a, V> {
    fn new(v: &'a mut V) -> Self {
        Self(v, false)
    }
    fn visited(&self) -> bool {
        self.1
    }
}
impl<'a, C, V> SingleCurrencyVisitor<C> for AnyCyrrencyVisitorImpl<'a, V>
where
    V: AnyCurrencyVisitor,
    C: Currency,
{
    fn on(&mut self, coin: Coin<C>) {
        self.0.on(coin);
        self.1 = true;
    }

    fn on_unknown(&mut self) {
        // delivers only on finish
    }
}

struct CoinTransformer<C>(Option<Coin<C>>);
impl<C> SingleCurrencyVisitor<C> for CoinTransformer<C>
where
    C: Currency,
{
    fn on(&mut self, coin: Coin<C>) {
        self.0 = Some(coin);
    }

    fn on_unknown(&mut self) {}
}

#[cfg(test)]
mod test {
    use std::{
        any::{type_name, TypeId},
        marker::PhantomData,
    };

    use crate::{coin_legacy, error::Error};

    use super::{AnyCurrencyVisitor, Coin, Currency, Nls, SingleCurrencyVisitor, Usdc};

    use cosmwasm_std::Coin as CosmWasmCoin;

    #[test]
    fn test_add() {
        let c1 = Coin::<Nls>::new(10);
        let c2 = Coin::<Nls>::new(20);
        let c12 = Coin::<Nls>::new(30);
        assert_eq!(c12, c1 + c2);
    }

    struct Expect<C>(PhantomData<C>, bool);
    impl<C> Expect<C> {
        fn new() -> Self {
            Self(PhantomData, false)
        }
        fn called(&self) -> bool {
            self.1
        }
    }
    impl<C> AnyCurrencyVisitor for Expect<C>
    where
        C: 'static,
    {
        fn on<Cin>(&mut self, _coin: Coin<Cin>)
        where
            Cin: 'static,
        {
            assert_eq!(
                TypeId::of::<C>(),
                TypeId::of::<Cin>(),
                "Expected {}, got {}",
                type_name::<C>(),
                type_name::<Cin>()
            );
            self.1 = true;
        }

        fn on_unknown(&mut self) {
            unreachable!();
        }
    }
    impl<C> SingleCurrencyVisitor<C> for Expect<C> {
        fn on(&mut self, _coin: Coin<C>) {
            self.1 = true;
        }

        fn on_unknown(&mut self) {
            unreachable!();
        }
    }

    struct ExpectUnknownCurrency(bool);
    impl ExpectUnknownCurrency {
        fn called(&self) -> bool {
            self.0
        }
    }
    impl AnyCurrencyVisitor for ExpectUnknownCurrency {
        fn on<C>(&mut self, _coin: Coin<C>)
        where
            C: Currency,
        {
            unreachable!();
        }

        fn on_unknown(&mut self) {
            self.0 = true;
        }
    }

    impl<C> SingleCurrencyVisitor<C> for ExpectUnknownCurrency {
        fn on(&mut self, _coin: Coin<C>) {
            unreachable!();
        }

        fn on_unknown(&mut self) {
            self.0 = true;
        }
    }
    #[test]
    fn visit_any() {
        let mut v_usdc = Expect::<Usdc>::new();
        coin_legacy::visit_any(&CosmWasmCoin::new(121, Usdc::SYMBOL), &mut v_usdc);
        assert!(v_usdc.called());

        let mut v_nls = Expect::<Nls>::new();
        coin_legacy::visit_any(&CosmWasmCoin::new(11, Nls::SYMBOL), &mut v_nls);
        assert!(v_nls.called());
    }

    #[test]
    fn visit_any_unexpected() {
        let mut v = ExpectUnknownCurrency(false);
        coin_legacy::visit_any(&CosmWasmCoin::new(0, "my_fancy_coin"), &mut v);
        assert!(v.called());
    }

    #[test]
    fn visit_one() {
        let mut v_usdc = Expect::<Usdc>::new();
        coin_legacy::visit(&CosmWasmCoin::new(121, Usdc::SYMBOL), &mut v_usdc);
        assert!(v_usdc.called());

        let mut v_nls = Expect::<Nls>::new();
        coin_legacy::visit(&CosmWasmCoin::new(11, Nls::SYMBOL), &mut v_nls);
        assert!(v_nls.called());
    }

    #[test]
    fn visit_one_unexpected() {
        let mut v = ExpectUnknownCurrency(false);
        coin_legacy::visit::<Nls, _>(&CosmWasmCoin::new(0, "my_fancy_coin"), &mut v);
        assert!(v.called());
    }

    #[test]
    fn from_cosmwasm() {
        let c1 = coin_legacy::from_cosmwasm::<Nls>(CosmWasmCoin::new(12, Nls::SYMBOL));
        assert_eq!(Ok(Coin::<Nls>::new(12)), c1);
    }
    #[test]
    fn from_cosmwasm_unexpected() {
        let c1 = coin_legacy::from_cosmwasm::<Nls>(CosmWasmCoin::new(12, Usdc::SYMBOL));
        assert_eq!(
            Err(Error::UnexpectedCurrency(
                Usdc::SYMBOL.into(),
                Nls::SYMBOL.into()
            )),
            c1
        );
        let c2 = coin_legacy::from_cosmwasm::<Usdc>(CosmWasmCoin::new(12, Nls::SYMBOL));
        assert_eq!(
            Err(Error::UnexpectedCurrency(
                Nls::SYMBOL.into(),
                Usdc::SYMBOL.into(),
            )),
            c2
        );
    }

    #[test]
    fn to_cosmwasm() {
        let amount = 326;
        assert_eq!(CosmWasmCoin::new(amount, Nls::SYMBOL), coin_legacy::to_cosmwasm(Coin::<Nls>::new(amount)));
        assert_eq!(CosmWasmCoin::new(amount, Usdc::SYMBOL), coin_legacy::to_cosmwasm(Coin::<Usdc>::new(amount)));
    }
}
