use std::marker::PhantomData;

use currencies::{LeaseGroup, Lpns, Native, PaymentOnlyGroup};
use tree::NodeRef;

use currency::{
    AnyVisitor, AnyVisitorResult, CurrencyDTO, CurrencyDef, Group, MaybeAnyVisitResult, MemberOf,
    never::{self, Never},
};

use crate::api::{Currency as ApiCurrency, CurrencyGroup, swap::SwapTarget};

pub fn currencies<'r, TopG, Nodes>(nodes: Nodes) -> impl Iterator<Item = ApiCurrency>
where
    TopG: Group + 'r,
    LeaseGroup: MemberOf<TopG>,
    Lpns: MemberOf<TopG>,
    Native: MemberOf<TopG>,
    PaymentOnlyGroup: MemberOf<TopG>,
    Nodes: Iterator<Item = NodeRef<'r, SwapTarget<TopG>>>,
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
) -> MaybeAnyVisitResult<G, CurrencyVisitor<G>>
where
    G: Group + MemberOf<TopG>,
    TopG: Group,
{
    node.value()
        .target
        .may_into_currency_type::<G, _>(CurrencyVisitor::<G>(PhantomData, api_group))
}

struct CurrencyVisitor<VisitedG>(PhantomData<VisitedG>, CurrencyGroup);

impl<VisitedG> AnyVisitor<VisitedG> for CurrencyVisitor<VisitedG>
where
    VisitedG: Group,
{
    type Output = ApiCurrency;

    type Error = Never;

    fn on<C>(self, def: &CurrencyDTO<C::Group>) -> AnyVisitorResult<VisitedG, Self>
    where
        C: CurrencyDef,
    {
        Ok(ApiCurrency::new(def.definition(), self.1))
    }
}
