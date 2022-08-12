use cosmwasm_std::{Addr, Deps, DepsMut, Env, MessageInfo, Response, Storage, Uint128};
use platform::batch::Batch;
use serde::{de::DeserializeOwned, Serialize};

use finance::coin::Coin;
use finance::currency::Currency;
use platform::bank::{self, BankAccount, BankStub};

use crate::error::ContractError;
use crate::event;
use crate::lpp::LiquidityPool;
use crate::msg::{BalanceResponse, PriceResponse};
use crate::state::Deposit;

pub fn try_deposit<LPN>(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError>
where
    LPN: 'static + Currency + DeserializeOwned + Serialize,
{
    let lender_addr = info.sender;
    let amount = bank::received(&info.funds)?;

    let lpp = LiquidityPool::<LPN>::load(deps.storage)?;

    let price = lpp.calculate_price(&deps.as_ref(), &env, amount)?;

    let receipts = Deposit::load_or_default(deps.storage, lender_addr.clone())?.deposit(
        deps.storage,
        amount,
        price,
    )?;

    Ok(event::emit_deposit(Batch::default(), env, lender_addr, amount, receipts).into())
}

pub fn try_withdraw<LPN>(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    amount_nlpn: Uint128,
) -> Result<Response, ContractError>
where
    LPN: 'static + Currency + DeserializeOwned + Serialize,
{
    let lender_addr = info.sender;
    let amount_nlpn = Coin::new(amount_nlpn.u128());

    let lpp = LiquidityPool::<LPN>::load(deps.storage)?;
    let payment_lpn = lpp.withdraw_lpn(&deps.as_ref(), &env, amount_nlpn)?;

    let maybe_reward = Deposit::may_load(deps.storage, lender_addr.clone())?
        .ok_or(ContractError::NoDeposit {})?
        .withdraw(deps.storage, amount_nlpn)?;

    let mut bank = BankStub::my_account(&env, &deps.querier);
    bank.send(payment_lpn, &lender_addr)?;

    if let Some(reward) = maybe_reward {
        if !reward.is_zero() {
            bank.send(reward, &lender_addr)?;
        }
    }

    let batch: Batch = bank.into();

    Ok(event::emit_withdraw(
        batch,
        env,
        lender_addr,
        payment_lpn,
        amount_nlpn,
        maybe_reward.is_some(),
    )
    .into())
}

pub fn query_ntoken_price<LPN>(deps: Deps, env: Env) -> Result<PriceResponse<LPN>, ContractError>
where
    LPN: Currency + DeserializeOwned + Serialize,
{
    let lpp = LiquidityPool::<LPN>::load(deps.storage)?;
    let price = lpp.calculate_price(&deps, &env, Coin::new(0))?.into();

    Ok(price)
}

pub fn query_balance(storage: &dyn Storage, addr: Addr) -> Result<BalanceResponse, ContractError> {
    let balance: u128 = Deposit::query_balance_nlpn(storage, addr)?
        .unwrap_or_default()
        .into();
    Ok(BalanceResponse {
        balance: balance.into(),
    })
}

// TODO: add more tests
#[cfg(test)]
mod test {
    use super::*;
    use cosmwasm_std::coin;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info, MOCK_CONTRACT_ADDR};
    use finance::currency::Usdc;
    use finance::price;
 

    type TheCurrency = Usdc;

    #[test]
    fn test_deposit() {
        let mut deps = mock_dependencies();
        let env = mock_env();

        let mut lpp_balance = 0;
        let init_deposit = 20_000;
        let lpp_balance_push = 80_000;
        let pushed_price = (lpp_balance_push + init_deposit) / init_deposit;
        let test_deposit = 10_004;
        let rounding_error = test_deposit % pushed_price; // should be 4 for this setup
        let post_deposit = 1_000_000;
        // let loan = 1_000_000;
        let overdraft = 5_000;
        let withdraw_amount_nlpn = 1000u128;
        let rest_nlpn = 1000u128;
        let zero = 0u128;

        LiquidityPool::<TheCurrency>::store(
            deps.as_mut().storage,
            TheCurrency::SYMBOL.into(),
            1000u64.into(),
        )
        .unwrap();

        // initial deposit
        lpp_balance += init_deposit;
        let info = mock_info("lender1", &[coin(init_deposit, TheCurrency::SYMBOL)]);
        deps.querier.update_balance(
            MOCK_CONTRACT_ADDR,
            vec![coin(lpp_balance, TheCurrency::SYMBOL)],
        );
        try_deposit::<TheCurrency>(deps.as_mut(), env.clone(), info).unwrap();

        // push the price from 1, should be allowed as an interest from previous leases for example.
        lpp_balance += lpp_balance_push;
        deps.querier.update_balance(
            MOCK_CONTRACT_ADDR,
            vec![coin(lpp_balance, TheCurrency::SYMBOL)],
        );

        let price = query_ntoken_price(deps.as_ref(), env.clone()).unwrap().0;
        assert_eq!(
            price::total(Coin::new(1_000), price),
            Coin::<TheCurrency>::new(1_000 * pushed_price)
        );

        // deposit to check,
        lpp_balance += test_deposit;
        let info = mock_info("lender2", &[coin(test_deposit, TheCurrency::SYMBOL)]);
        deps.querier.update_balance(
            MOCK_CONTRACT_ADDR,
            vec![coin(lpp_balance, TheCurrency::SYMBOL)],
        );
        try_deposit::<TheCurrency>(deps.as_mut(), env.clone(), info).unwrap();

        // got rounding error
        let balance_nlpn = query_balance(deps.as_ref().storage, Addr::unchecked("lender2"))
            .unwrap()
            .balance;
        let price = query_ntoken_price(deps.as_ref(), env.clone()).unwrap().0;
        assert_eq!(
            Coin::<TheCurrency>::new(test_deposit - rounding_error),
            price::total(balance_nlpn.into(), price)
        );

        // should not change asserts for lender2
        lpp_balance += post_deposit;
        let info = mock_info("lender3", &[coin(post_deposit, TheCurrency::SYMBOL)]);
        deps.querier.update_balance(
            MOCK_CONTRACT_ADDR,
            vec![coin(lpp_balance, TheCurrency::SYMBOL)],
        );
        try_deposit::<TheCurrency>(deps.as_mut(), env.clone(), info).unwrap();

        let balance_nlpn = query_balance(deps.as_ref().storage, Addr::unchecked("lender2"))
            .unwrap()
            .balance;
        let price = query_ntoken_price(deps.as_ref(), env.clone()).unwrap().0;
        assert_eq!(
            Coin::<TheCurrency>::new(test_deposit - rounding_error),
            price::total(balance_nlpn.into(), price)
        );

        //try to deposit zero
        let info = mock_info("lender4", &[coin(zero, TheCurrency::SYMBOL)]);
        let result = try_deposit::<TheCurrency>(deps.as_mut(), env.clone(), info);
        assert!(result.is_err());

        // try to withdraw with overdraft
        let info = mock_info("lender2", &[]);
        let result = try_withdraw::<TheCurrency>(
            deps.as_mut(),
            env.clone(),
            info.clone(),
            (test_deposit - rounding_error + overdraft).into(),
        );
        assert!(result.is_err());

        //try to withdraw zero
        try_withdraw::<TheCurrency>(deps.as_mut(), env.clone(), info.clone(), zero.into()).unwrap_err();
      
    

        // partial withdraw
        try_withdraw::<TheCurrency>(
            deps.as_mut(),
            env.clone(),
            info.clone(),
            withdraw_amount_nlpn.into(),
        )
        .unwrap();
        let balance_nlpn = query_balance(deps.as_ref().storage, Addr::unchecked("lender2"))
            .unwrap()
            .balance;
        assert_eq!(balance_nlpn, rest_nlpn.into());

        // full withdraw
        try_withdraw::<TheCurrency>(deps.as_mut(), env, info, rest_nlpn.into()).unwrap();
        let balance_nlpn = query_balance(deps.as_ref().storage, Addr::unchecked("lender2"))
            .unwrap()
            .balance;
        assert_eq!(balance_nlpn, zero.into());
    }
}
