use cosmwasm_std::{
    coins,
    testing::{mock_env, MockApi},
    to_binary, Addr, Api, Binary, BlockInfo, CanonicalAddr, Coin, Deps, Env, RecoverPubkeyError,
    StdResult, Timestamp, VerificationError,
};
use cw_multi_test::{App, AppBuilder, BankKeeper};
use serde::{Deserialize, Serialize};

use currency::native::Nls;
use finance::{currency::Currency, duration::Duration};

type ContractWrapper<
    ExecMsg,
    ExecErr,
    InstMsg,
    InstErr,
    QueryMsg,
    QueryErr,
    Sudo = cosmwasm_std::Empty,
    SudoErr = anyhow::Error,
    ReplyErr = anyhow::Error,
    MigrMsg = cosmwasm_std::Empty,
    MigrErr = anyhow::Error,
> = cw_multi_test::ContractWrapper<
    ExecMsg,             // execute msg
    InstMsg,             // instantiate msg
    QueryMsg,            // query msg
    ExecErr,             // execute err
    InstErr,             // instantiate err
    QueryErr,            // query err
    cosmwasm_std::Empty, // C
    cosmwasm_std::Empty, // Q
    Sudo,                // sudo msg
    SudoErr,             // sudo err
    ReplyErr,            // reply err
    MigrMsg,             // migrate msg
    MigrErr,             // migrate err
>;

#[cfg(test)]
#[allow(dead_code)]
pub mod dispatcher_wrapper;
pub mod lease_wrapper;
#[cfg(test)]
pub mod leaser_wrapper;
#[cfg(test)]
#[allow(dead_code)]
pub mod lpp_wrapper;
pub mod oracle_wrapper;
pub mod profit_wrapper;
pub mod timealarms_wrapper;

#[cfg(test)]
pub mod test_case;
pub mod treasury_wrapper;

pub const USER: &str = "user";
pub const ADMIN: &str = "admin";
pub const NATIVE_DENOM: &str = Nls::SYMBOL;

#[derive(Serialize, Clone, Debug, PartialEq)]
struct MockResponse {}
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
struct MockQueryMsg {}

#[derive(Default)]
pub struct ApiWithNullAddresses<A>(A)
where
    A: Api;

impl<A> From<A> for ApiWithNullAddresses<A>
where
    A: Api,
{
    fn from(api: A) -> Self {
        Self(api)
    }
}

impl<A> Api for ApiWithNullAddresses<A>
where
    A: Api,
{
    fn addr_validate(&self, human: &str) -> StdResult<Addr> {
        if human.is_empty() {
            Ok(Addr::unchecked(String::default()))
        } else {
            self.0.addr_validate(human)
        }
    }

    fn addr_canonicalize(&self, human: &str) -> StdResult<CanonicalAddr> {
        self.0.addr_canonicalize(human)
    }

    fn addr_humanize(&self, canonical: &CanonicalAddr) -> StdResult<Addr> {
        self.0.addr_humanize(canonical)
    }

    fn secp256k1_verify(
        &self,
        message_hash: &[u8],
        signature: &[u8],
        public_key: &[u8],
    ) -> Result<bool, VerificationError> {
        self.0.secp256k1_verify(message_hash, signature, public_key)
    }

    fn secp256k1_recover_pubkey(
        &self,
        message_hash: &[u8],
        signature: &[u8],
        recovery_param: u8,
    ) -> Result<Vec<u8>, RecoverPubkeyError> {
        self.0
            .secp256k1_recover_pubkey(message_hash, signature, recovery_param)
    }

    fn ed25519_verify(
        &self,
        message: &[u8],
        signature: &[u8],
        public_key: &[u8],
    ) -> Result<bool, VerificationError> {
        self.0.ed25519_verify(message, signature, public_key)
    }

    fn ed25519_batch_verify(
        &self,
        messages: &[&[u8]],
        signatures: &[&[u8]],
        public_keys: &[&[u8]],
    ) -> Result<bool, VerificationError> {
        self.0
            .ed25519_batch_verify(messages, signatures, public_keys)
    }

    fn debug(&self, message: &str) {
        self.0.debug(message)
    }
}

fn mock_query(_deps: Deps, _env: Env, _msg: MockQueryMsg) -> StdResult<Binary> {
    to_binary(&MockResponse {})
}

pub type MockApp = App<BankKeeper, ApiWithNullAddresses<MockApi>>;

pub fn mock_app(init_funds: &[Coin]) -> MockApp {
    let return_time = mock_env().block.time.minus_seconds(400 * 24 * 60 * 60);

    let mock_start_block = BlockInfo {
        height: 12_345,
        time: return_time,
        chain_id: "cosmos-testnet-14002".to_string(),
    };

    let mut funds = coins(1000, NATIVE_DENOM);
    funds.append(&mut init_funds.to_vec());

    AppBuilder::new()
        .with_block(mock_start_block)
        .with_api(ApiWithNullAddresses::from(MockApi::default()))
        .build(|router, _, storage| {
            router
                .bank
                .init_balance(storage, &Addr::unchecked(ADMIN), funds)
                .unwrap();
        })
}

pub trait AppExt {
    fn time_shift(&mut self, t: Duration);
}

impl AppExt for MockApp {
    fn time_shift(&mut self, t: Duration) {
        self.update_block(|block| {
            let ct = block.time.nanos();
            block.time = Timestamp::from_nanos(ct + t.nanos());
            block.height += 1;
        })
    }
}
