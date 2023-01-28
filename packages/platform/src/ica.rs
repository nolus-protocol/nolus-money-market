use prost::Message;
use serde::{Deserialize, Serialize};

use finance::{coin::Coin, currency::Currency, duration::Duration};
use sdk::neutron_sdk::bindings::{
    msg::{IbcFee, NeutronMsg},
    types::ProtobufAny,
};

use crate::{
    batch::Batch,
    coin_legacy,
    error::{Error, Result},
};

use self::impl_::OpenAckVersion;

/// Identifier of the ICA account opened by a lease
/// It is unique for a lease and allows the support of multiple accounts per lease
const ICA_ACCOUNT_ID: &str = "0";

/// ICA Host Account
///
/// Holds the address on the ICA host network
#[derive(Clone, Serialize, Deserialize)]
pub struct HostAccount(String);
impl TryFrom<String> for HostAccount {
    type Error = Error;
    fn try_from(addr: String) -> Result<Self> {
        if addr.is_empty() {
            Err(Error::InvalidICAHostAccount())
        } else {
            Ok(Self(addr))
        }
    }
}

impl From<HostAccount> for String {
    fn from(account: HostAccount) -> Self {
        account.0
    }
}

pub fn register_account<C>(connection: C) -> Batch
where
    C: Into<String>,
{
    let mut batch = Batch::default();
    batch.schedule_execute_no_reply(NeutronMsg::register_interchain_account(
        connection.into(),
        ICA_ACCOUNT_ID.into(),
    ));
    batch
}

pub fn parse_register_response(response: &str) -> Result<HostAccount> {
    let open_ack = serde_json_wasm::from_str::<OpenAckVersion>(response)?;
    open_ack.address.try_into()
}

pub fn submit_transaction<Conn, M, C>(
    connection: Conn,
    trx: Transaction,
    memo: M,
    timeout: Duration,
    ack_tip: Coin<C>,
    timeout_tip: Coin<C>,
) -> Batch
where
    Conn: Into<String>,
    M: Into<String>,
    C: Currency,
{
    let mut batch = Batch::default();

    batch.schedule_execute_no_reply(NeutronMsg::submit_tx(
        connection.into(),
        ICA_ACCOUNT_ID.into(),
        trx.msgs,
        memo.into(),
        timeout.secs(),
        IbcFee {
            recv_fee: vec![],
            ack_fee: vec![coin_legacy::to_cosmwasm_impl(ack_tip)],
            timeout_fee: vec![coin_legacy::to_cosmwasm_impl(timeout_tip)],
        },
    ));
    batch
}

#[derive(Default)]
#[cfg_attr(any(test, feature = "testing"), derive(Debug, PartialEq))]
pub struct Transaction {
    msgs: Vec<ProtobufAny>,
}

impl Transaction {
    pub fn add_message<T, M>(&mut self, msg_type: T, msg: M)
    where
        T: Into<String>,
        M: Message,
    {
        let mut buf = Vec::with_capacity(msg.encoded_len());
        msg.encode_raw(&mut buf);

        self.msgs
            .push(ProtobufAny::new(msg_type.into(), buf.into()));
    }
}

mod impl_ {
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize, Debug)]
    #[serde(rename_all = "snake_case")]
    pub struct OpenAckVersion {
        pub version: String,
        pub controller_connection_id: String,
        pub host_connection_id: String,
        pub address: String,
        pub encoding: String,
        pub tx_type: String,
    }
}
