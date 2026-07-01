mod execute;
mod instantiate;

use currencies::Nls;
use currency::CurrencyDef as _;
use finance::coin::Amount;
use sdk::{
    cosmwasm_std::{
        Coin as CwCoin, DepsMut, MessageInfo, OwnedDeps,
        testing::{self, MockApi, MockQuerier, MockStorage},
    },
    testing as sdk_testing,
};

use crate::{api::InstantiateMsg, contract::instantiate};

const OWNER: &str = "profit";

fn deps() -> OwnedDeps<MockStorage, MockApi, MockQuerier> {
    sdk_testing::mock_deps_with_contracts([])
}

fn sender(who: &str) -> MessageInfo {
    MessageInfo {
        sender: sdk_testing::user(who),
        funds: vec![],
    }
}

fn instantiate_default(deps: DepsMut<'_>) {
    instantiate(
        deps,
        testing::mock_env(),
        sender("creator"),
        InstantiateMsg {
            owner: sdk_testing::user(OWNER).into_string(),
        },
    )
    .expect("instantiation succeeds");
}

/// Seed the vault contract address with the given `unls` balance.
fn fund_vault(deps: &mut OwnedDeps<MockStorage, MockApi, MockQuerier>, amount: Amount) {
    let vault = testing::mock_env().contract.address;
    deps.querier.bank.update_balance(
        vault,
        vec![CwCoin::new(amount, Nls::dto().definition().bank_symbol)],
    );
}
