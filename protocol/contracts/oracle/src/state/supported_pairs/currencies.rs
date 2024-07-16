use currencies::{LeaseGroup, Lpns, Native, PaymentGroup, PaymentOnlyGroup};
use tree::NodeRef;

use currency::{
    never::{self, Never},
    AnyVisitor, AnyVisitorResult, Currency, GroupVisit, Tickers,
};

use crate::api::{swap::SwapTarget, Currency as ApiCurrency, CurrencyGroup};

pub fn currencies<'r, Nodes>(nodes: Nodes) -> impl Iterator<Item = ApiCurrency> + 'r
where
    Nodes: Iterator<Item = NodeRef<'r, SwapTarget>> + 'r,
{
    nodes.map(|node| {
        never::safe_unwrap(
            Tickers::maybe_visit_any(&node.value().target, CurrencyVisitor()).unwrap_or_else(
                |_err| unreachable!("The payment group does not cover all available currencies!"),
            ),
        )
    })
}

struct CurrencyVisitor();

impl AnyVisitor for CurrencyVisitor {
    type VisitedG = PaymentGroup;

    type Output = ApiCurrency;

    type Error = Never;

    fn on<C>(self) -> AnyVisitorResult<Self>
    where
        C: Currency,
    {
        let group = if currency::equal::<LeaseGroup, C::Group>() {
            CurrencyGroup::Lease
        } else if currency::equal::<Lpns, C::Group>() {
            CurrencyGroup::Lpn
        } else if currency::equal::<Native, C::Group>() {
            CurrencyGroup::Native
        } else {
            debug_assert!(currency::equal::<PaymentOnlyGroup, C::Group>());
            CurrencyGroup::PaymentOnly
        };
        Ok(ApiCurrency::new::<C>(group))
    }
}
