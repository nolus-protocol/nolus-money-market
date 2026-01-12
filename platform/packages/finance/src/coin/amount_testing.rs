use crate::{coin::Amount, fractionable::ToDoublePrimitive};
use crate::{
    fractionable::{CommonDoublePrimitive, Fractionable},
    percent::{Units as PercentUnits, bound::BoundPercent},
};

// TODO Remove once integration tests use BoundPercent::of(Coin)
impl<const UPPER_BOUND: PercentUnits> CommonDoublePrimitive<BoundPercent<UPPER_BOUND>> for Amount {
    type CommonDouble = <Self as ToDoublePrimitive>::Double;
}

// TODO Remove once integration tests use BoundPercent::of(Coin)
impl<const UPPER_BOUND: PercentUnits> Fractionable<BoundPercent<UPPER_BOUND>> for Amount {}
