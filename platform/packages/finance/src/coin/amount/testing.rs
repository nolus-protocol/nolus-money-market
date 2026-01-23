use crate::{coin::Amount, fractionable::ToDoublePrimitive};
use crate::{
    fractionable::{CommonDoublePrimitive, Fractionable},
    percent::{Units as PercentUnits, bound::BoundPercent},
};

// These implementations exist strictly to be used for integration test purposes

impl<const UPPER_BOUND: PercentUnits> CommonDoublePrimitive<BoundPercent<UPPER_BOUND>> for Amount {
    type CommonDouble = <Self as ToDoublePrimitive>::Double;
}

impl<const UPPER_BOUND: PercentUnits> Fractionable<BoundPercent<UPPER_BOUND>> for Amount {}
