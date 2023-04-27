use crate::percent::Percent;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(any(test, feature = "testing"), derive(Debug))]
pub enum Level {
    First(Percent),
    Second(Percent),
    Third(Percent),
    Max(Percent),
}

impl Level {
    pub fn ltv(&self) -> Percent {
        *match self {
            Self::First(ltv) | Self::Second(ltv) | Self::Third(ltv) | Self::Max(ltv) => ltv,
        }
    }

    pub fn ordinal(self) -> u8 {
        match self {
            Self::First(_) => 1,
            Self::Second(_) => 2,
            Self::Third(_) => 3,
            Self::Max(_) => 4,
        }
    }
}

impl From<Level> for Percent {
    fn from(value: Level) -> Self {
        value.ltv()
    }
}
