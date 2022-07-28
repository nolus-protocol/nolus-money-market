use cosmwasm_std::{Addr, Deps, DepsMut, Env, MessageInfo, Response, Storage, Uint128, Event};
use platform::batch::Batch;
use serde::{de::DeserializeOwned, Serialize};

use finance::coin::Coin;
use finance::currency::Currency;
use platform::bank::{self, BankAccount, BankStub};

use crate::error::ContractError;
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

    let receipts =  finance::price::total(amount, price.get().inv());


    Deposit::load(deps.storage, lender_addr.clone())?.deposit(deps.storage, amount, price)?;



    let transaction_idx = match env.transaction {
        Some(idx) => idx.index.to_string(),
        None => String::from(""),
      };
    
    let deposit_event = Event::new("wasm-lp-deposit")
    .add_attribute("height", env.block.height.to_string())
    .add_attribute("idx", transaction_idx)
    .add_attribute("from", lender_addr)
    .add_attribute("at", env.block.time.to_string())
    .add_attribute("to", env.contract.address)
    .add_attribute("amount", info.funds[0].amount.to_string())
    .add_attribute("currency", info.funds[0].denom.to_string())
    .add_attribute("receipts", receipts.to_string());


   Ok(Response::new().add_attribute("method", "try_deposit").add_event(deposit_event))
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

    let maybe_reward =
        Deposit::load(deps.storage, lender_addr.clone())?.withdraw(deps.storage, amount_nlpn)?;

    let mut bank = BankStub::my_account(&env, &deps.querier);
    bank.send(payment_lpn, &lender_addr);

    if let Some(reward) = maybe_reward {
        bank.send(reward, &lender_addr);
    }
    let batch: Batch = bank.into();

    let mut response: Response = batch.into();
    response = response.add_attribute("method", "try_withdraw");
    Ok(response)
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
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info, MOCK_CONTRACT_ADDR};
    use cosmwasm_std::coin;
    use finance::currency::Usdc;
    use finance::price;

    type TheCurrency = Usdc;

    #[test]
    fn test_deposit() {
        let mut deps = mock_dependencies();
        let env = mock_env();

        LiquidityPool::<TheCurrency>::store(deps.as_mut().storage, TheCurrency::SYMBOL.into(), 1000u64.into())
            .unwrap();

        let info = mock_info("lender1", &[coin(20000, TheCurrency::SYMBOL)]);
        deps.querier.update_balance(MOCK_CONTRACT_ADDR, vec![coin(20000, TheCurrency::SYMBOL)]);
        try_deposit::<TheCurrency>(deps.as_mut(), env.clone(), info).unwrap();

        let info = mock_info("lender2", &[coin(10000, TheCurrency::SYMBOL)]);
        deps.querier.update_balance(MOCK_CONTRACT_ADDR, vec![coin(30000, TheCurrency::SYMBOL)]);
        try_deposit::<TheCurrency>(deps.as_mut(), env.clone(), info).unwrap();

        let lpp = LiquidityPool::<TheCurrency>::load(deps.as_ref().storage).unwrap();
        let price = lpp.calculate_price(&deps.as_ref(), &env, Coin::new(0)).unwrap();

        let balance_nlpn = Deposit::query_balance_nlpn(deps.as_ref().storage, Addr::unchecked("lender2")).unwrap().unwrap();
        assert_eq!(Coin::<TheCurrency>::new(10000), price::total(balance_nlpn, price.get()));
    }
}
