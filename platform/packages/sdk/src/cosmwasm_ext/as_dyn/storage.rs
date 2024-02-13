#[cfg(any(test, feature = "testing"))]
use cosmwasm_std::testing::MockStorage;
use cosmwasm_std::Storage;

use super::{AsDyn, AsDynMut, HigherOrderDyn};

pub trait Dyn: AsDyn<dyn Storage> {}

impl<T> Dyn for T where T: AsDyn<dyn Storage> + ?Sized {}

pub trait DynMut: AsDynMut<dyn Storage> {}

impl<T> DynMut for T where T: AsDynMut<dyn Storage> + ?Sized {}

impl HigherOrderDyn for dyn Storage {
    type Dyn<'r> = dyn Storage + 'r where Self: 'r;
}

impl<'r> AsDyn<dyn Storage> for dyn Storage + 'r {
    fn as_dyn(&self) -> &(dyn Storage + '_) {
        self
    }
}

impl<'r> AsDynMut<dyn Storage> for dyn Storage + 'r {
    fn as_dyn_mut(&mut self) -> &mut (dyn Storage + '_) {
        self
    }
}

#[cfg(feature = "testing")]
impl AsDyn<dyn Storage> for MockStorage {
    fn as_dyn(&self) -> &(dyn Storage + '_) {
        self
    }
}

#[cfg(feature = "testing")]
impl AsDynMut<dyn Storage> for MockStorage {
    fn as_dyn_mut(&mut self) -> &mut (dyn Storage + '_) {
        self
    }
}

#[cfg(test)]
#[test]
fn test_impls() {
    let mut storage: MockStorage = MockStorage::new();

    let storage: &mut dyn Storage = &mut storage;

    let _ = storage.as_dyn();
    let _ = storage.as_dyn_mut();

    let storage: &dyn Storage = storage;

    let _ = storage.as_dyn();
}
