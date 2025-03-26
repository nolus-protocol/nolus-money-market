use crate::{Account, Enterable};

/// Entity expecting to be connected to ICA
pub trait IcaConnectee {
    type State;
    type NextState: Enterable + Into<Self::State>;

    fn connected(self, ica_account: Account) -> Self::NextState;
}
