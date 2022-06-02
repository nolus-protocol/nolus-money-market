use std::ops::{Mul, Div};

use cosmwasm_std::{Uint256, Coin};

use crate::percent::Percent;

pub trait Percentable: Mul<Percent, Output = <Self as Percentable>::Intermediate> {
    type Intermediate: Div<Percent, Output = <Self as Percentable>::Result>;
    type Result: Percentable;
}

impl Percentable for Coin {
    type Intermediate = Coin256;
    type Result = Self;
}

impl Percentable for &Coin {
    type Intermediate = Coin256;
    type Result = Coin;
}

pub struct Coin256 {
    pub denom: String,
    pub amount: Uint256,
}

impl Mul<Percent> for &Coin {
    type Output = Coin256;

    fn mul(self, rhs: Percent) -> Self::Output {
        self.clone().mul(rhs)
    }
}

impl Mul<Percent> for Coin {
    type Output = Coin256;

    fn mul(self, rhs: Percent) -> Self::Output {
        Self::Output {
            denom: self.denom,
            amount: Uint256::from(self.amount).mul(Uint256::from(rhs.units())),
        }
    }
}

impl Div<Percent> for Coin256 {
    type Output = Coin;

    fn div(self, rhs: Percent) -> Self::Output {
        let amount256 = self.amount.div(Uint256::from(rhs.units()));
        Self::Output {
            denom: self.denom,
            amount: amount256.try_into().expect("Overflow computing percent"),
        }
    }
}