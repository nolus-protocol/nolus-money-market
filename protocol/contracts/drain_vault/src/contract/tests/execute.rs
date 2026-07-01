use currencies::Nls;
use currency::CurrencyDef as _;
use sdk::{
    cosmwasm_ext::CosmosMsg,
    cosmwasm_std::{Addr, BankMsg, Coin as CwCoin, testing},
    testing as sdk_testing,
};

use crate::{api::ExecuteMsg, contract::execute, error::Error};

use super::{OWNER, deps, fund_vault, instantiate_default, sender};

const BALANCE: u128 = 7_000;

fn recipient() -> Addr {
    sdk_testing::user("home")
}

#[test]
fn sweep_moves_full_balance_to_target() {
    let mut deps = deps();
    instantiate_default(deps.as_mut());
    fund_vault(&mut deps, BALANCE);

    let res = execute(
        deps.as_mut(),
        testing::mock_env(),
        sender(OWNER),
        ExecuteMsg::Sweep {
            recipient: recipient(),
        },
    )
    .expect("the owner sweep succeeds");

    assert_eq!(1, res.messages.len(), "expected exactly one bank send");
    match &res.messages[0].msg {
        CosmosMsg::Bank(BankMsg::Send { to_address, amount }) => {
            assert_eq!(&recipient().into_string(), to_address);
            assert_eq!(
                &vec![CwCoin::new(BALANCE, Nls::dto().definition().bank_symbol)],
                amount,
                "the full balance is swept, with no reserve carve-out"
            );
        }
        other => panic!("expected CosmosMsg::Bank(BankMsg::Send {{..}}), got {other:?}"),
    }
}

#[test]
fn sweep_empty_balance_is_a_no_op() {
    let mut deps = deps();
    instantiate_default(deps.as_mut());
    fund_vault(&mut deps, 0);

    let res = execute(
        deps.as_mut(),
        testing::mock_env(),
        sender(OWNER),
        ExecuteMsg::Sweep {
            recipient: recipient(),
        },
    )
    .expect("an empty sweep is a no-op, not an error");

    assert!(res.messages.is_empty(), "a zero balance emits no bank send");
}

#[test]
fn non_owner_sweep_rejected() {
    let mut deps = deps();
    instantiate_default(deps.as_mut());
    fund_vault(&mut deps, BALANCE);

    let err = execute(
        deps.as_mut(),
        testing::mock_env(),
        sender("intruder"),
        ExecuteMsg::Sweep {
            recipient: recipient(),
        },
    )
    .unwrap_err();

    assert!(
        matches!(err, Error::Unauthorized(_)),
        "a non-owner sender is rejected, got {err:?}"
    );
}
