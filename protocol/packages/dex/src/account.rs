use serde::{Deserialize, Serialize};

use platform::remote::Account as RemoteAccount;
use sdk::cosmwasm_std::Addr;

use crate::{Connectable, ConnectionParams};

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct Account {
    /// The contract at Nolus that owns the account
    owner: Addr,
    // converted from the Remote Lease Id, used as destination for outgoing transfers
    // cannot use Remote Account Id because `dex` is protocol-agnostic
    remote: RemoteAccount,
    /// The Remote Lease controller contract that opened this account,
    /// retained for the upcoming callback-authorization path.
    remote_controller: Addr,
    dex: ConnectionParams,
}

impl Account {
    pub fn owner(&self) -> &Addr {
        &self.owner
    }

    pub fn remote(&self) -> &RemoteAccount {
        &self.remote
    }

    pub fn new(
        owner: Addr,
        remote: RemoteAccount,
        remote_controller: Addr,
        dex: ConnectionParams,
    ) -> Self {
        Self {
            owner,
            remote,
            remote_controller,
            dex,
        }
    }
}

impl Connectable for Account {
    fn dex(&self) -> &ConnectionParams {
        &self.dex
    }
}

#[cfg(test)]
mod test {
    use platform::remote::Account as RemoteAccount;
    use sdk::cosmwasm_std::{Addr, from_json, to_json_string};

    use crate::{ConnectionParams, Ics20Channel};

    use super::Account;

    #[test]
    fn new_serializes_expected_wire_shape() {
        assert_eq!(
            r#"{"owner":"owner-contract","remote":"remote-account","remote_controller":"remote-lease-controller","dex":{"connection_id":"connection-0","transfer_channel":{"local_endpoint":"channel-0","remote_endpoint":"channel-2048"}}}"#,
            to_json_string(&account()).unwrap()
        );
    }

    #[test]
    fn serde_round_trip() {
        let serialized = to_json_string(&account()).unwrap();
        let restored: Account = from_json(&serialized).unwrap();
        assert_eq!(serialized, to_json_string(&restored).unwrap());
    }

    fn account() -> Account {
        Account::new(
            Addr::unchecked("owner-contract"),
            RemoteAccount::try_from("remote-account".to_string()).unwrap(),
            Addr::unchecked("remote-lease-controller"),
            ConnectionParams {
                connection_id: "connection-0".into(),
                transfer_channel: Ics20Channel {
                    local_endpoint: "channel-0".into(),
                    remote_endpoint: "channel-2048".into(),
                },
            },
        )
    }
}
