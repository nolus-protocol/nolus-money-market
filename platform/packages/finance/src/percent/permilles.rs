use std::fmt::{Display, Formatter, Result as FmtResult};

use serde::{Deserialize, Serialize};

use crate::percent::Units;

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct Permilles(Units);

impl Permilles {
    pub(super) const ZERO: Self = Self::new(0);
    pub(super) const PRECISION: Self = Self::new(1);
    pub(super) const MILLE: Self = Self::new(super::HUNDRED);

    pub const fn new(permilles: Units) -> Self {
        Self(permilles)
    }

    pub(super) const fn units(&self) -> Units {
        self.0
    }

    pub(super) const fn checked_add(self, other: Self) -> Option<Self> {
        if let Some(res) = self.0.checked_add(other.0) {
            Some(Self(res))
        } else {
            None
        }
    }

    pub(super) const fn checked_sub(self, other: Self) -> Option<Self> {
        if let Some(res) = self.0.checked_sub(other.0) {
            Some(Self(res))
        } else {
            None
        }
    }
}

impl Display for Permilles {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        f.write_fmt(format_args!("{}‰", self.0))
    }
}

#[cfg(test)]
mod test {
    use crate::percent::{Units, permilles::Permilles};

    #[test]
    fn display() {
        test_display("0‰", 0);
        test_display("10‰", 10);
        test_display("100‰", 100);
        test_display("127‰", 127);
    }

    fn test_display(exp: &str, permilles: Units) {
        assert_eq!(exp, format!("{}", Permilles(permilles)));
    }
}
