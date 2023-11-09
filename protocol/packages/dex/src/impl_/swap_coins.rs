use currency::Group;
use finance::coin::CoinDTO;

use super::swap_task::{CoinVisitor, IterNext, IterState};

#[cfg(test)]
pub(super) use self::test::TestVisitor;

pub fn on_coin<G, Visitor>(
    coin: &CoinDTO<G>,
    visitor: &mut Visitor,
) -> Result<IterState, Visitor::Error>
where
    G: Group,
    Visitor: CoinVisitor,
{
    visitor.visit(coin).map(|_iter_next| IterState::Complete)
}

pub fn on_coins<G1, G2, Visitor>(
    coin1: &CoinDTO<G1>,
    coin2: &CoinDTO<G2>,
    visitor: &mut Visitor,
) -> Result<IterState, Visitor::Error>
where
    G1: Group,
    G2: Group,
    Visitor: CoinVisitor<Result = IterNext>,
{
    visitor.visit(coin1).and_then(|next| match next {
        IterNext::Continue => on_coin(coin2, visitor),
        IterNext::Stop => Ok(IterState::Incomplete),
    })
}

#[cfg(test)]
mod test {
    use currency::{
        test::{SubGroup, SuperGroup, SuperGroupTestC1, SuperGroupTestC2},
        Group,
    };

    use finance::coin::{Amount, Coin, CoinDTO};
    use platform::never::{self, Never};

    use crate::impl_::swap_task::{CoinVisitor, IterNext, IterState};

    fn coin1() -> CoinDTO<SuperGroup> {
        Coin::<SuperGroupTestC1>::new(32).into()
    }

    fn coin2() -> CoinDTO<SubGroup> {
        Coin::<SuperGroupTestC2>::new(28).into()
    }

    pub struct TestVisitor<R>(Option<Amount>, R, Option<Amount>, R);
    impl<R> TestVisitor<R> {
        pub fn first_visited(&self, a: Amount) -> bool {
            self.0.map_or(false, |a_visit| a == a_visit)
        }
        pub fn first_not_visited(&self) -> bool {
            self.0.is_none()
        }
        pub fn second_visited(&self, a: Amount) -> bool {
            self.2.map_or(false, |a_visit| a == a_visit)
        }
        pub fn second_not_visited(&self) -> bool {
            self.2.is_none()
        }
    }
    impl TestVisitor<IterNext> {
        pub fn new(r1: IterNext, r2: IterNext) -> Self {
            Self(None, r1, None, r2)
        }
    }
    impl TestVisitor<()> {
        pub fn new() -> Self {
            Self(None, (), None, ())
        }
    }
    impl<R> CoinVisitor for TestVisitor<R>
    where
        R: Clone,
    {
        type Result = R;
        type Error = Never;

        fn visit<G>(&mut self, coin: &CoinDTO<G>) -> Result<Self::Result, Self::Error>
        where
            G: Group,
        {
            assert!(self.2.is_none());
            let res = if self.0.is_none() {
                self.0 = Some(coin.amount());
                self.1.clone()
            } else {
                self.2 = Some(coin.amount());
                self.3.clone()
            };
            Ok(res)
        }
    }

    #[test]
    fn visit_one() {
        let mut v = TestVisitor::<()>::new();
        let iter_res = never::safe_unwrap(super::on_coin(&coin1(), &mut v));
        assert_eq!(iter_res, IterState::Complete);
        assert!(v.first_visited(coin1().amount()));
        assert!(v.second_not_visited());
    }

    #[test]
    fn visit_two_stop_one() {
        let mut v = TestVisitor::<IterNext>::new(IterNext::Stop, IterNext::Continue);

        let iter_res = never::safe_unwrap(super::on_coins(&coin1(), &coin2(), &mut v));
        assert_eq!(iter_res, IterState::Incomplete);
        assert!(v.first_visited(coin1().amount()));
        assert!(v.second_not_visited());
    }

    #[test]
    fn visit_two_stop_two() {
        let mut v = TestVisitor::<IterNext>::new(IterNext::Continue, IterNext::Stop);

        let iter_res = never::safe_unwrap(super::on_coins(&coin2(), &coin1(), &mut v));
        assert_eq!(iter_res, IterState::Complete);
        assert!(v.first_visited(coin2().amount()));
        assert!(v.second_visited(coin1().amount()));
    }

    #[test]
    fn visit_two_continue() {
        let mut v = TestVisitor::<IterNext>::new(IterNext::Continue, IterNext::Continue);

        let iter_res = never::safe_unwrap(super::on_coins(&coin1(), &coin2(), &mut v));
        assert_eq!(iter_res, IterState::Complete);
        assert!(v.first_visited(coin1().amount()));
        assert!(v.second_visited(coin2().amount()));
    }
}
