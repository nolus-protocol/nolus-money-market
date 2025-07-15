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
