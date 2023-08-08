use serde::{de::DeserializeOwned, Serialize};

use currency::Currency;
use finance::coin::Coin;
use platform::{
    bank::{self, BankAccount},
    batch::Batch,
    message::Response as MessageResponse,
};
use sdk::cosmwasm_std::{Addr, Deps, DepsMut, Env, MessageInfo, Storage, Uint128};

use crate::{
    error::{ContractError, Result},
    event,
    lpp::LiquidityPool,
    msg::{BalanceResponse, PriceResponse},
    state::Deposit,
};

pub(super) fn try_deposit<Lpn>(
    deps: DepsMut<'_>,
    env: Env,
    info: MessageInfo,
) -> Result<MessageResponse>
where
    Lpn: 'static + Currency + DeserializeOwned + Serialize,
{
    let lender_addr = info.sender;
    let amount = bank::received_one(info.funds)?;

    let lpp = LiquidityPool::<Lpn>::load(deps.storage)?;

    lpp.check_utilization_rate(&deps.querier, &env)?;

    let price = lpp.calculate_price(&deps.as_ref(), &env, amount)?;

    let receipts = Deposit::load_or_default(deps.storage, lender_addr.clone())?.deposit(
        deps.storage,
        amount,
        price,
    )?;

    Ok(event::emit_deposit(env, lender_addr, amount, receipts).into())
}

pub(super) fn try_withdraw<Lpn>(
    deps: DepsMut<'_>,
    env: Env,
    info: MessageInfo,
    amount_nlpn: Uint128,
) -> Result<MessageResponse>
where
    Lpn: 'static + Currency + DeserializeOwned + Serialize,
{
    if amount_nlpn.is_zero() {
        return Err(ContractError::ZeroWithdrawFunds);
    }

    let lender_addr = info.sender;
    let amount_nlpn = Coin::new(amount_nlpn.u128());

    let lpp = LiquidityPool::<Lpn>::load(deps.storage)?;
    let payment_lpn = lpp.withdraw_lpn(&deps.as_ref(), &env, amount_nlpn)?;

    let maybe_reward = Deposit::may_load(deps.storage, lender_addr.clone())?
        .ok_or(ContractError::NoDeposit {})?
        .withdraw(deps.storage, amount_nlpn)?;

    let mut bank = bank::account(&env.contract.address, &deps.querier);
    bank.send(payment_lpn, &lender_addr);

    if let Some(reward) = maybe_reward {
        if !reward.is_zero() {
            bank.send(reward, &lender_addr);
        }
    }

    let batch: Batch = bank.into();
    Ok(MessageResponse::messages_with_events(
        batch,
        event::emit_withdraw(
            env,
            lender_addr,
            payment_lpn,
            amount_nlpn,
            maybe_reward.is_some(),
        ),
    ))
}

pub fn query_ntoken_price<Lpn>(deps: Deps<'_>, env: Env) -> Result<PriceResponse<Lpn>>
where
    Lpn: Currency + DeserializeOwned + Serialize,
{
    let lpp = LiquidityPool::<Lpn>::load(deps.storage)?;
    let price = lpp.calculate_price(&deps, &env, Coin::new(0))?.into();

    Ok(price)
}

pub fn query_balance(storage: &dyn Storage, addr: Addr) -> Result<BalanceResponse> {
    let balance: u128 = Deposit::query_balance_nlpn(storage, addr)?
        .unwrap_or_default()
        .into();
    Ok(BalanceResponse {
        balance: balance.into(),
    })
}

#[cfg(test)]
mod test {
    use access_control::ContractOwnerAccess;
    use currency::lpn::Usdc;
    use finance::{percent::Percent, price};
    use platform::coin_legacy;
    use sdk::cosmwasm_std::{
        testing::{mock_dependencies, mock_env, mock_info, MOCK_CONTRACT_ADDR},
        Coin as CwCoin,
    };

    use crate::{borrow::InterestRate, state::Config};

    use super::*;

    type TheCurrency = Usdc;

    const BASE_INTEREST_RATE: Percent = Percent::from_permille(70);
    const UTILIZATION_OPTIMAL: Percent = Percent::from_permille(700);
    const ADDON_OPTIMAL_INTEREST_RATE: Percent = Percent::from_permille(20);

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

        ContractOwnerAccess::new(deps.as_mut().storage)
            .grant_to(&Addr::unchecked("admin"))
            .unwrap();

        LiquidityPool::<TheCurrency>::store(
            deps.as_mut().storage,
            Config::new(
                TheCurrency::TICKER.into(),
                1000u64.into(),
                InterestRate::new(
                    BASE_INTEREST_RATE,
                    UTILIZATION_OPTIMAL,
                    ADDON_OPTIMAL_INTEREST_RATE,
                )
                .expect("Couldn't construct interest rate value!"),
                Percent::from_permille(250),
            ),
        )
        .unwrap();

        // initial deposit
        lpp_balance += init_deposit;
        let info = mock_info("lender1", &[cwcoin(init_deposit)]);
        deps.querier
            .update_balance(MOCK_CONTRACT_ADDR, vec![cwcoin(lpp_balance)]);
        try_deposit::<TheCurrency>(deps.as_mut(), env.clone(), info).unwrap();

        // push the price from 1, should be allowed as an interest from previous leases for example.
        lpp_balance += lpp_balance_push;
        deps.querier
            .update_balance(MOCK_CONTRACT_ADDR, vec![cwcoin(lpp_balance)]);

        let price = query_ntoken_price(deps.as_ref(), env.clone()).unwrap().0;
        assert_eq!(
            price::total(Coin::new(1_000), price),
            Coin::<TheCurrency>::new(1_000 * pushed_price)
        );

        // deposit to check,
        lpp_balance += test_deposit;
        let info = mock_info("lender2", &[cwcoin(test_deposit)]);
        deps.querier
            .update_balance(MOCK_CONTRACT_ADDR, vec![cwcoin(lpp_balance)]);
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
        let info = mock_info("lender3", &[cwcoin(post_deposit)]);
        deps.querier
            .update_balance(MOCK_CONTRACT_ADDR, vec![cwcoin(lpp_balance)]);
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
        let info = mock_info("lender4", &[cwcoin(zero)]);
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

    fn cwcoin<A>(amount: A) -> CwCoin
    where
        A: Into<Coin<TheCurrency>>,
    {
        coin_legacy::to_cosmwasm::<TheCurrency>(amount.into())
    }
}
