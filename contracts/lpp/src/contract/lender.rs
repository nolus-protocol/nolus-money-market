use serde::{de::DeserializeOwned, Serialize};

use finance::{coin::Coin, currency::Currency};
use platform::{
    bank::{self, BankAccount},
    batch::Batch,
};
use sdk::{
    cosmwasm_ext::Response,
    cosmwasm_std::{Addr, Deps, DepsMut, Env, MessageInfo, Storage, Uint128},
};

use crate::{
    error::ContractError,
    event,
    lpp::LiquidityPool,
    msg::{BalanceResponse, PriceResponse},
    state::Deposit,
};

pub fn try_deposit<LPN>(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError>
where
    LPN: 'static + Currency + DeserializeOwned + Serialize,
{
    let lender_addr = info.sender;
    let amount = bank::received(info.funds)?;

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
    if amount_nlpn.is_zero() {
        return Err(ContractError::ZeroWithdrawFunds);
    }

    let lender_addr = info.sender;
    let amount_nlpn = Coin::new(amount_nlpn.u128());

    let lpp = LiquidityPool::<LPN>::load(deps.storage)?;
    let payment_lpn = lpp.withdraw_lpn(&deps.as_ref(), &env, amount_nlpn)?;

    let maybe_reward = Deposit::may_load(deps.storage, lender_addr.clone())?
        .ok_or(ContractError::NoDeposit {})?
        .withdraw(deps.storage, amount_nlpn)?;

    let mut bank = bank::my_account(&env, &deps.querier);
    bank.send(payment_lpn, &lender_addr);

    if let Some(reward) = maybe_reward {
        if !reward.is_zero() {
            bank.send(reward, &lender_addr);
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

#[cfg(test)]
mod test {
    use access_control::SingleUserAccess;
    use currency::lpn::Usdc;
    use finance::price;
    use platform::coin_legacy;
    use sdk::cosmwasm_std::{
        testing::{mock_dependencies, mock_env, mock_info, MOCK_CONTRACT_ADDR},
        Coin as CwCoin,
    };

    use super::*;

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
        let overdraft = 5_000;
        let withdraw_amount_nlpn = 1000u128;
        let rest_nlpn = 1000u128;
        let zero = 0u128;

        SingleUserAccess::new(
            crate::access_control::OWNER_NAMESPACE,
            Addr::unchecked("admin"),
        )
        .store(deps.as_mut().storage)
        .unwrap();

        LiquidityPool::<TheCurrency>::store(
            deps.as_mut().storage,
            TheCurrency::TICKER.into(),
            1000u64.into(),
        )
        .unwrap();

        // initial deposit
        lpp_balance += init_deposit;
        let info = mock_info("lender1", &cwcoins(init_deposit));
        deps.querier
            .update_balance(MOCK_CONTRACT_ADDR, cwcoins(lpp_balance));
        try_deposit::<TheCurrency>(deps.as_mut(), env.clone(), info).unwrap();

        // push the price from 1, should be allowed as an interest from previous leases for example.
        lpp_balance += lpp_balance_push;
        deps.querier
            .update_balance(MOCK_CONTRACT_ADDR, cwcoins(lpp_balance));

        let price = query_ntoken_price(deps.as_ref(), env.clone()).unwrap().0;
        assert_eq!(
            price::total(Coin::new(1_000), price),
            Coin::<TheCurrency>::new(1_000 * pushed_price)
        );

        // deposit to check,
        lpp_balance += test_deposit;
        let info = mock_info("lender2", &cwcoins(test_deposit));
        deps.querier
            .update_balance(MOCK_CONTRACT_ADDR, cwcoins(lpp_balance));
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
        let info = mock_info("lender3", &cwcoins(post_deposit));
        deps.querier
            .update_balance(MOCK_CONTRACT_ADDR, cwcoins(lpp_balance));
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
        let info = mock_info("lender4", &cwcoins(zero));
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
        let result =
            try_withdraw::<TheCurrency>(deps.as_mut(), env.clone(), info.clone(), zero.into());
        assert!(result.is_err());

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
        assert_eq!(balance_nlpn.u128(), rest_nlpn);

        // full withdraw
        try_withdraw::<TheCurrency>(deps.as_mut(), env, info, rest_nlpn.into()).unwrap();
        let balance_nlpn = query_balance(deps.as_ref().storage, Addr::unchecked("lender2"))
            .unwrap()
            .balance;
        assert_eq!(balance_nlpn.u128(), zero);
    }

    fn cwcoins<A>(amount: A) -> Vec<CwCoin>
    where
        A: Into<Coin<TheCurrency>>,
    {
        vec![coin_legacy::to_cosmwasm::<TheCurrency>(amount.into())]
    }
}
