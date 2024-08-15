use std::marker::PhantomData;

use currencies::{LeaseGroup, Lpns, Native, PaymentOnlyGroup};
use tree::NodeRef;

use currency::{
    never::{self, Never},
    AnyVisitor, AnyVisitorResult, CurrencyDef, Group, MaybeAnyVisitResult, MemberOf,
};

use crate::api::{swap::SwapTarget, Currency as ApiCurrency, CurrencyGroup};

pub fn currencies<'r, TopG, Nodes>(nodes: Nodes) -> impl Iterator<Item = ApiCurrency> + 'r
where
    TopG: Group + 'r,
    LeaseGroup: MemberOf<TopG>,
    Lpns: MemberOf<TopG>,
    Native: MemberOf<TopG>,
    PaymentOnlyGroup: MemberOf<TopG>,
    Nodes: Iterator<Item = NodeRef<'r, SwapTarget<TopG>>> + 'r,
{
    nodes.map(|node| {
        never::safe_unwrap(
            map_from_group::<LeaseGroup, TopG>(node, CurrencyGroup::Lease)
                .or_else(|_err| map_from_group::<Lpns, TopG>(node, CurrencyGroup::Lpn))
                .or_else(|_err| map_from_group::<Native, TopG>(node, CurrencyGroup::Native))
                .or_else(|_err| {
                    map_from_group::<PaymentOnlyGroup, TopG>(node, CurrencyGroup::PaymentOnly)
                })
                .unwrap_or_else(|_err| {
                    unreachable!("The payment group does not cover all available currencies!")
                }),
        )
    })
}

fn map_from_group<G, TopG>(
    node: NodeRef<'_, SwapTarget<TopG>>,
    api_group: CurrencyGroup,
) -> MaybeAnyVisitResult<G, CurrencyVisitor<G, TopG>>
where
    G: Group + MemberOf<TopG>,
    TopG: Group,
{
    node.value()
        .target
        .may_into_currency_type::<G, _>(CurrencyVisitor::<G, TopG>(
            PhantomData,
            PhantomData,
            api_group,
        ))
}

struct CurrencyVisitor<VisitedG, VisitorG>(
    PhantomData<VisitedG>,
    PhantomData<VisitorG>,
    CurrencyGroup,
);

impl<VisitedG, VisitorG> AnyVisitor<VisitedG> for CurrencyVisitor<VisitedG, VisitorG>
where
    VisitedG: Group + MemberOf<VisitorG>,
    VisitorG: Group,
{
    type VisitorG = VisitorG;

    type Output = ApiCurrency;

    type Error = Never;

    fn on<C>(self, def: &C) -> AnyVisitorResult<VisitedG, Self>
    where
        C: CurrencyDef,
    {
        // TODO get rid of visiting
        Ok(ApiCurrency::new(def.dto().definition(), self.2))
    }
}
