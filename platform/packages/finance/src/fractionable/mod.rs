use crate::{duration::Units as TimeUnits, percent::Units as PercentUnits, ratio::Ratio};

mod coin;
mod duration;
mod percent;
mod price;
mod usize;

pub trait Fractionable<U> {
    #[track_caller]
    fn safe_mul<F>(self, fraction: &F) -> Self
    where
        F: Ratio<U>;
}

// TODO revisit its usability
pub trait Percentable: Fractionable<PercentUnits> {}
pub trait TimeSliceable: Fractionable<TimeUnits> {}

pub trait HigherRank<T> {
    type Type;
    // An intermediate type to handle cases when there is no TryInto<Self> for HigherRank::Type but
    // instead there is TryInto<HigherRank::Intermediate> for HigherRank::Type, and Into<Self> for HigherRank::Intermediate
    type Intermediate;
}
