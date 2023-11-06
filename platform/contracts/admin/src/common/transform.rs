use std::collections::BTreeMap;

pub(crate) trait Transform
where
    Self: Sized,
{
    type Context<'r>: ?Sized;

    type Output;

    type Error;

    fn transform(self, ctx: &Self::Context<'_>) -> Result<Self::Output, Self::Error>;
}

pub(crate) trait Map
where
    Self: IntoIterator<Item = (Self::Key, Self::Value)>,
{
    type HigherOrderSelf<K, V>;

    type Key;

    type Value;
}

impl<K, V> Map for BTreeMap<K, V>
where
    K: Ord,
{
    type HigherOrderSelf<K2, V2> = BTreeMap<K2, V2>;

    type Key = K;

    type Value = V;
}

pub(crate) struct TransformByValue<M>(M)
where
    M: Map,
    M::HigherOrderSelf<M::Key, <M::Value as Transform>::Output>:
        FromIterator<(M::Key, <M::Value as Transform>::Output)>,
    M::Value: Transform;

impl<M> TransformByValue<M>
where
    M: Map,
    M::HigherOrderSelf<M::Key, <M::Value as Transform>::Output>:
        FromIterator<(M::Key, <M::Value as Transform>::Output)>,
    M::Value: Transform,
{
    pub const fn new(map: M) -> Self {
        Self(map)
    }
}

impl<M> Transform for TransformByValue<M>
where
    M: Map,
    M::HigherOrderSelf<M::Key, <M::Value as Transform>::Output>:
        FromIterator<(M::Key, <M::Value as Transform>::Output)>,
    M::Value: Transform,
{
    type Context<'r> = <M::Value as Transform>::Context<'r>;

    type Output = M::HigherOrderSelf<M::Key, <M::Value as Transform>::Output>;

    type Error = <M::Value as Transform>::Error;

    fn transform(self, ctx: &Self::Context<'_>) -> Result<Self::Output, Self::Error> {
        self.0
            .into_iter()
            .map(|(key, value): (M::Key, M::Value)| {
                value
                    .transform(ctx)
                    .map(|value: <M::Value as Transform>::Output| (key, value))
            })
            .collect::<Result<_, _>>()
    }
}
