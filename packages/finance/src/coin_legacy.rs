use crate::{
    coin::{Coin, Currency, Nls, Usdc},
    error::{Error, Result},
};
use cosmwasm_std::{Coin as CosmWasmCoin, Uint128};

pub fn sub_amount(from: CosmWasmCoin, amount: Uint128) -> CosmWasmCoin {
    CosmWasmCoin {
        amount: from.amount - amount,
        denom: from.denom,
    }
}

pub fn add_coin(to: CosmWasmCoin, other: CosmWasmCoin) -> CosmWasmCoin {
    debug_assert!(to.denom == other.denom);
    CosmWasmCoin {
        amount: to.amount + other.amount,
        denom: to.denom,
    }
}

pub trait AnyDenomVisitor {
    fn on<C>(&mut self, coin: Coin<C>)
    where
        C: Currency;
    fn on_unknown(&mut self);
}

pub trait SingleDenomVisitor<C> {
    fn on(&mut self, coin: Coin<C>);
    fn on_unknown(&mut self);
}

pub fn visit_denom_any<V>(coin: &CosmWasmCoin, visitor: &mut V)
where
    V: AnyDenomVisitor,
{
    let mut any_visitor = AnyDenomVisitorImpl::new(visitor);
    debug_assert!(!any_visitor.visited());
    visit_denom_one::<Nls, _>(coin, &mut any_visitor);
    if !any_visitor.visited() {
        visit_denom_one::<Usdc, _>(coin, &mut any_visitor);
    }
    if !any_visitor.visited() {
        visitor.on_unknown();
    }
}

pub fn visit_denom_one<C, V>(coin: &CosmWasmCoin, visitor: &mut V)
where
    V: SingleDenomVisitor<C>,
    C: Currency,
{
    let amount: u128 = coin.amount.into();
    let denom = coin.denom.as_str();
    if denom == C::DENOM {
        visitor.on(Coin::<C>::new(amount));
    } else {
        visitor.on_unknown();
    }
}

pub fn from_cosmwasm<C>(coin: CosmWasmCoin) -> Result<Coin<C>>
where
    C: Currency,
{
    let mut v = CoinTransformer(None);
    visit_denom_one(&coin, &mut v);
    v.0.ok_or_else(|| Error::UnexpectedCurrency(coin.denom, C::DENOM.into()))
}

struct AnyDenomVisitorImpl<'a, V>(&'a mut V, bool);
impl<'a, V> AnyDenomVisitorImpl<'a, V> {
    fn new(v: &'a mut V) -> Self {
        Self(v, false)
    }
    fn visited(&self) -> bool {
        self.1
    }
}
impl<'a, C, V> SingleDenomVisitor<C> for AnyDenomVisitorImpl<'a, V>
where
    V: AnyDenomVisitor,
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
impl<C> SingleDenomVisitor<C> for CoinTransformer<C>
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

    use super::{AnyDenomVisitor, Coin, Currency, Nls, SingleDenomVisitor, Usdc};

    use cosmwasm_std::Coin as CosmWasmCoin;

    #[test]
    fn test_add() {
        let c1 = Coin::<Nls>::new(10);
        let c2 = Coin::<Nls>::new(20);
        let c12 = Coin::<Nls>::new(30);
        assert_eq!(c12, c1 + c2);
    }

    struct ExpectDenom<C>(PhantomData<C>, bool);
    impl<C> ExpectDenom<C> {
        fn new() -> Self {
            Self(PhantomData, false)
        }
        fn called(&self) -> bool {
            self.1
        }
    }
    impl<C> AnyDenomVisitor for ExpectDenom<C>
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
    impl<C> SingleDenomVisitor<C> for ExpectDenom<C> {
        fn on(&mut self, _coin: Coin<C>) {
            self.1 = true;
        }

        fn on_unknown(&mut self) {
            unreachable!();
        }
    }

    struct ExpectUnknownDenom(bool);
    impl ExpectUnknownDenom {
        fn called(&self) -> bool {
            self.0
        }
    }
    impl AnyDenomVisitor for ExpectUnknownDenom {
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

    impl<C> SingleDenomVisitor<C> for ExpectUnknownDenom {
        fn on(&mut self, _coin: Coin<C>) {
            unreachable!();
        }

        fn on_unknown(&mut self) {
            self.0 = true;
        }
    }
    #[test]
    fn visit_denom_any() {
        let mut v_usdc = ExpectDenom::<Usdc>::new();
        coin_legacy::visit_denom_any(&CosmWasmCoin::new(121, Usdc::DENOM), &mut v_usdc);
        assert!(v_usdc.called());

        let mut v_nls = ExpectDenom::<Nls>::new();
        coin_legacy::visit_denom_any(&CosmWasmCoin::new(11, Nls::DENOM), &mut v_nls);
        assert!(v_nls.called());
    }

    #[test]
    fn visit_denom_any_unexpected() {
        let mut v = ExpectUnknownDenom(false);
        coin_legacy::visit_denom_any(&CosmWasmCoin::new(0, "my_fancy_coin"), &mut v);
        assert!(v.called());
    }

    #[test]
    fn visit_denom_one() {
        let mut v_usdc = ExpectDenom::<Usdc>::new();
        coin_legacy::visit_denom_one(&CosmWasmCoin::new(121, Usdc::DENOM), &mut v_usdc);
        assert!(v_usdc.called());

        let mut v_nls = ExpectDenom::<Nls>::new();
        coin_legacy::visit_denom_one(&CosmWasmCoin::new(11, Nls::DENOM), &mut v_nls);
        assert!(v_nls.called());
    }

    #[test]
    fn visit_denom_one_unexpected() {
        let mut v = ExpectUnknownDenom(false);
        coin_legacy::visit_denom_one::<Nls, _>(&CosmWasmCoin::new(0, "my_fancy_coin"), &mut v);
        assert!(v.called());
    }

    #[test]
    fn from_cosmwasm() {
        let c1 = coin_legacy::from_cosmwasm::<Nls>(CosmWasmCoin::new(12, Nls::DENOM));
        assert_eq!(Ok(Coin::<Nls>::new(12)), c1);
    }
    #[test]
    fn from_cosmwasm_unexpected() {
        let c1 = coin_legacy::from_cosmwasm::<Nls>(CosmWasmCoin::new(12, Usdc::DENOM));
        assert_eq!(
            Err(Error::UnexpectedCurrency(
                Usdc::DENOM.into(),
                Nls::DENOM.into()
            )),
            c1
        );
        let c2 = coin_legacy::from_cosmwasm::<Usdc>(CosmWasmCoin::new(12, Nls::DENOM));
        assert_eq!(
            Err(Error::UnexpectedCurrency(
                Nls::DENOM.into(),
                Usdc::DENOM.into(),
            )),
            c2
        );
    }
}
