use std::array;

use serde::{Deserialize, Serialize};

use sdk::schemars::{self, JsonSchema};

pub(crate) mod type_defs;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct CodeIdWithMigrateMsg<M> {
    pub code_id: u64,
    pub migrate_msg: M,
}

pub(crate) trait Contracts {
    type Item;

    type SelfWith<T>: Contracts<Item = T>;

    type ZipIter<T>: Iterator<Item = (Self::Item, T)>;

    fn as_ref(&self) -> Self::SelfWith<&Self::Item>;

    fn as_mut(&mut self) -> Self::SelfWith<&mut Self::Item>;

    fn try_for_each<F, E>(self, f: F) -> Result<(), E>
    where
        F: FnMut(Self::Item) -> Result<(), E>;

    fn zip_iter<T>(self, other: Self::SelfWith<T>) -> Self::ZipIter<T>;
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct GeneralContracts<T> {
    pub dispatcher: T,
    pub leaser: T,
    pub profit: T,
    pub timealarms: T,
    pub treasury: T,
}

impl<T> Contracts for GeneralContracts<T> {
    type Item = T;

    type SelfWith<U> = GeneralContracts<U>;

    type ZipIter<U> = array::IntoIter<(T, U), 5>;

    fn as_ref(&self) -> GeneralContracts<&T> {
        GeneralContracts {
            dispatcher: &self.dispatcher,
            leaser: &self.leaser,
            profit: &self.profit,
            timealarms: &self.timealarms,
            treasury: &self.treasury,
        }
    }

    fn as_mut(&mut self) -> Self::SelfWith<&mut T> {
        GeneralContracts {
            dispatcher: &mut self.dispatcher,
            leaser: &mut self.leaser,
            profit: &mut self.profit,
            timealarms: &mut self.timealarms,
            treasury: &mut self.treasury,
        }
    }

    fn try_for_each<F, E>(self, f: F) -> Result<(), E>
    where
        F: FnMut(T) -> Result<(), E>,
    {
        [
            self.dispatcher,
            self.leaser,
            self.profit,
            self.timealarms,
            self.treasury,
        ]
        .into_iter()
        .try_for_each(f)
    }

    fn zip_iter<U>(self, other: Self::SelfWith<U>) -> Self::ZipIter<U> {
        [
            (self.dispatcher, other.dispatcher),
            (self.leaser, other.leaser),
            (self.profit, other.profit),
            (self.timealarms, other.timealarms),
            (self.treasury, other.treasury),
        ]
        .into_iter()
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct LpnContracts<T> {
    pub lpp: T,
    pub oracle: T,
}

impl<T> Contracts for LpnContracts<T> {
    type Item = T;

    type SelfWith<U> = LpnContracts<U>;

    type ZipIter<U> = array::IntoIter<(T, U), 2>;

    fn as_ref(&self) -> LpnContracts<&T> {
        LpnContracts {
            lpp: &self.lpp,
            oracle: &self.oracle,
        }
    }

    fn as_mut(&mut self) -> LpnContracts<&mut T> {
        LpnContracts {
            lpp: &mut self.lpp,
            oracle: &mut self.oracle,
        }
    }

    fn try_for_each<F, E>(self, f: F) -> Result<(), E>
    where
        F: FnMut(T) -> Result<(), E>,
    {
        [self.lpp, self.oracle].into_iter().try_for_each(f)
    }

    fn zip_iter<U>(self, other: LpnContracts<U>) -> Self::ZipIter<U> {
        [(self.lpp, other.lpp), (self.oracle, other.oracle)].into_iter()
    }
}
