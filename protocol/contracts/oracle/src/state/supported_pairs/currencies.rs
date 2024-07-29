use std::marker::PhantomData;

use currencies::{LeaseGroup, Lpns, Native, PaymentGroup, PaymentOnlyGroup};
use tree::NodeRef;

use currency::{
    never::{self, Never},
    AnyVisitor, AnyVisitorResult, Currency, Group, GroupVisit, MaybeAnyVisitResult, MemberOf,
    Tickers,
};

use crate::api::{swap::SwapTarget, Currency as ApiCurrency, CurrencyGroup};

pub fn currencies<'r, Nodes>(nodes: Nodes) -> impl Iterator<Item = ApiCurrency> + 'r
where
    Nodes: Iterator<Item = NodeRef<'r, SwapTarget>> + 'r,
{
    nodes.map(|node| {
        never::safe_unwrap(
            map_from_group::<LeaseGroup>(node, CurrencyGroup::Lease)
                .or_else(|_err| map_from_group::<Lpns>(node, CurrencyGroup::Lpn))
                .or_else(|_err| map_from_group::<Native>(node, CurrencyGroup::Native))
                .or_else(|_err| {
                    map_from_group::<PaymentOnlyGroup>(node, CurrencyGroup::PaymentOnly)
                })
                .unwrap_or_else(|_err| {
                    unreachable!("The payment group does not cover all available currencies!")
                }),
        )
    })
}

fn map_from_group<G>(
    node: NodeRef<'_, SwapTarget>,
    api_group: CurrencyGroup,
) -> MaybeAnyVisitResult<G, CurrencyVisitor<G>>
where
    G: Group + MemberOf<PaymentGroup>,
{
    Tickers::maybe_visit_member_any(
        &node.value().target,
        CurrencyVisitor::<G>(PhantomData, api_group),
    )
}

struct CurrencyVisitor<VisitedG>(PhantomData<VisitedG>, CurrencyGroup);

impl<VisitedG> AnyVisitor<VisitedG> for CurrencyVisitor<VisitedG>
where
    VisitedG: Group + MemberOf<PaymentGroup>,
{
    type VisitorG = PaymentGroup;

    type Output = ApiCurrency;

    type Error = Never;

    fn on<C>(self) -> AnyVisitorResult<VisitedG, Self>
    where
        C: Currency,
    {
        Ok(ApiCurrency::new::<C>(self.1))
    }
}
