use serde::{de::DeserializeOwned, Serialize};

use currency::Currency;
use platform::{
    bank::{self, BankAccount},
    batch::Batch,
    message::Response as MessageResponse,
};
use sdk::cosmwasm_std::{Addr, Deps, DepsMut, Env, MessageInfo, Storage};

use crate::{
    error::{ContractError, Result},
    lpp::LiquidityPool,
    msg::{LppBalanceResponse, RewardsResponse},
    state::Deposit,
};

pub(super) fn try_distribute_rewards(
    deps: DepsMut<'_>,
    info: MessageInfo,
) -> Result<MessageResponse> {
    bank::received_one(info.funds)
        .map_err(Into::into)
        .and_then(|amount| Deposit::distribute_rewards(deps, amount))
        .map(|()| Default::default())
}

pub(super) fn try_claim_rewards(
    deps: DepsMut<'_>,
    env: Env,
    info: MessageInfo,
    other_recipient: Option<Addr>,
) -> Result<MessageResponse> {
    let recipient = other_recipient
        .map(|recipient| deps.api.addr_validate(recipient.as_str()))
        .transpose()?
        .unwrap_or_else(|| info.sender.clone());

    let mut deposit =
        Deposit::may_load(deps.storage, info.sender)?.ok_or(ContractError::NoDeposit {})?;

    let reward = deposit.claim_rewards(deps.storage)?;

    if reward.is_zero() {
        return Err(ContractError::NoRewards {});
    }

    let mut bank = bank::account(&env.contract.address, &deps.querier);
    bank.send(reward, &recipient);
    let batch: Batch = bank.into();

    Ok(batch.into())
}

pub(super) fn query_lpp_balance<LPN>(deps: Deps<'_>, env: Env) -> Result<LppBalanceResponse<LPN>>
where
    LPN: 'static + Currency + DeserializeOwned + Serialize,
{
    let lpp = LiquidityPool::<LPN>::load(deps.storage)?;
    lpp.query_lpp_balance(&deps, &env)
}

pub(super) fn query_rewards(storage: &dyn Storage, addr: Addr) -> Result<RewardsResponse> {
    let rewards = Deposit::may_load(storage, addr)?
        .ok_or(ContractError::NoDeposit {})?
        .query_rewards(storage)?;

    Ok(RewardsResponse { rewards })
}

#[cfg(test)]
mod test {
    use access_control::SingleUserAccess;
    use currency::test::Usdc;
    use finance::{coin::Coin, percent::Percent};
    use platform::coin_legacy;
    use sdk::cosmwasm_std::{
        testing::{mock_dependencies, mock_env, mock_info, MOCK_CONTRACT_ADDR},
        Coin as CwCoin,
    };

    use crate::{borrow::InterestRate, contract::lender, state::Config};

    use super::*;

    type TheCurrency = Usdc;

    const BASE_INTEREST_RATE: Percent = Percent::from_permille(70);
    const UTILIZATION_OPTIMAL: Percent = Percent::from_permille(700);
    const ADDON_OPTIMAL_INTEREST_RATE: Percent = Percent::from_permille(20);

    #[test]
    fn test_claim_zero_rewards() {
        let mut deps = mock_dependencies();
        let env = mock_env();

        let mut lpp_balance = 0;
        let deposit = 20_000;

        SingleUserAccess::new_contract_owner(Addr::unchecked("admin"))
            .store(deps.as_mut().storage)
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
            ),
        )
        .unwrap();

        // no deposit
        let info = mock_info("lender", &[]);
        let response = try_claim_rewards(deps.as_mut(), env.clone(), info, None);
        assert_eq!(response, Err(ContractError::NoDeposit {}));

        lpp_balance += deposit;
        let info = mock_info("lender", &[cwcoin(deposit)]);
        deps.querier
            .update_balance(MOCK_CONTRACT_ADDR, vec![cwcoin(lpp_balance)]);
        lender::try_deposit::<TheCurrency>(deps.as_mut(), env.clone(), info).unwrap();

        // pending rewards == 0
        let info = mock_info("lender", &[]);
        let response = try_claim_rewards(deps.as_mut(), env, info, None);
        assert_eq!(response, Err(ContractError::NoRewards {}));
    }

    fn cwcoin<A>(amount: A) -> CwCoin
    where
        A: Into<Coin<TheCurrency>>,
    {
        coin_legacy::to_cosmwasm::<TheCurrency>(amount.into())
    }
}
