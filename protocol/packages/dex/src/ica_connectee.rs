use sdk::cosmwasm_std::Addr;

use crate::{Account, Enterable};

/// Entity expecting to be connected to ICA
pub trait IcaConnectee {
    type State;
    type NextState: Enterable + Into<Self::State>;

    fn connected(self, ica_account: Account) -> Self::NextState;

    /// The remote-lease controller authorised to dispatch a
    /// `RemoteLeaseCallback` to the connectee, when one applies. Connectees
    /// that do not participate in the remote-lease protocol leave the
    /// default (`None`); inbound callbacks are rejected for them.
    fn remote_lease(&self) -> Option<&Addr> {
        None
    }
}
