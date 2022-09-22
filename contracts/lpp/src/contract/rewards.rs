use cosmwasm_std::{Addr, Deps, DepsMut, Env, MessageInfo, Response, Storage};
use currency::native::Nls;
use serde::{de::DeserializeOwned, Serialize};

use finance::{coin::Coin, currency::Currency};
use platform::{
    bank::{self, BankAccount, BankStub},
    batch::Batch,
};

use crate::{
    error::ContractError,
    lpp::LiquidityPool,
    msg::{LppBalanceResponse, RewardsResponse},
    state::Deposit,
};

pub fn try_distribute_rewards(deps: DepsMut, info: MessageInfo) -> Result<Response, ContractError> {
    let amount: Coin<Nls> = bank::received(info.funds)?;
    Deposit::distribute_rewards(deps, amount)?;

    Ok(Response::new().add_attribute("method", "try_distribute_rewards"))
}

pub fn try_claim_rewards(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    other_recipient: Option<Addr>,
) -> Result<Response, ContractError> {
    let recipient = other_recipient.unwrap_or_else(|| info.sender.clone());
    let mut deposit =
        Deposit::may_load(deps.storage, info.sender)?.ok_or(ContractError::NoDeposit {})?;

    let reward = deposit.claim_rewards(deps.storage)?;

    if reward.is_zero() {
        return Err(ContractError::NoRewards {});
    }

    let mut bank = BankStub::my_account(&env, &deps.querier);
    bank.send(reward, &recipient);

    let batch: Batch = bank.into();

    let mut batch: Response = batch.into();
    batch = batch.add_attribute("method", "try_claim_rewards");
    Ok(batch)
}

pub fn query_lpp_balance<LPN>(
    deps: Deps,
    env: Env,
) -> Result<LppBalanceResponse<LPN>, ContractError>
where
    LPN: 'static + Currency + DeserializeOwned + Serialize,
{
    let lpp = LiquidityPool::<LPN>::load(deps.storage)?;
    lpp.query_lpp_balance(&deps, &env)
}

pub fn query_rewards(storage: &dyn Storage, addr: Addr) -> Result<RewardsResponse, ContractError> {
    let rewards = Deposit::may_load(storage, addr)?
        .ok_or(ContractError::NoDeposit {})?
        .query_rewards(storage)?;

    Ok(RewardsResponse { rewards })
}

#[cfg(test)]
mod test {
    use cosmwasm_std::{
        coin,
        testing::{mock_dependencies, mock_env, mock_info, MOCK_CONTRACT_ADDR},
    };
    use finance::test::currency::Usdc;

    use crate::contract::lender;

    use super::*;

    type TheCurrency = Usdc;

    #[test]
    fn test_claim_zero_rewards() {
        let mut deps = mock_dependencies();
        let env = mock_env();

        let mut lpp_balance = 0;
        let deposit = 20_000;

        LiquidityPool::<TheCurrency>::store(
            deps.as_mut().storage,
            TheCurrency::SYMBOL.into(),
            1000u64.into(),
        )
        .unwrap();

        // no deposit
        let info = mock_info("lender", &[]);
        let response = try_claim_rewards(deps.as_mut(), env.clone(), info, None);
        assert_eq!(response, Err(ContractError::NoDeposit {}));

        lpp_balance += deposit;
        let info = mock_info("lender", &[coin(deposit, TheCurrency::SYMBOL)]);
        deps.querier.update_balance(
            MOCK_CONTRACT_ADDR,
            vec![coin(lpp_balance, TheCurrency::SYMBOL)],
        );
        lender::try_deposit::<TheCurrency>(deps.as_mut(), env.clone(), info).unwrap();

        // pending rewards == 0
        let info = mock_info("lender", &[]);
        let response = try_claim_rewards(deps.as_mut(), env, info, None);
        assert_eq!(response, Err(ContractError::NoRewards {}));
    }
}
