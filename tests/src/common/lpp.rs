use currencies::Lpns;
use currency::{CurrencyDef, MemberOf};
use finance::percent::Percent100;
use lpp::{
    borrow::InterestRate,
    contract::{ContractError, sudo},
    msg::{ExecuteMsg, InstantiateMsg, QueryMsg},
};
use platform::contract::{Code, CodeId};
use sdk::{
    cosmwasm_std::{Addr, Binary, Coin as CwCoin, Deps, Env, to_json_binary},
    testing::{self, CwContract},
};

use super::{
    ADMIN, CwContractWrapper, leaser::Instantiator as LeaserInstantiator, test_case::app::App,
};

pub type LppExecuteMsg = ExecuteMsg<Lpns>;
pub type LppQueryMsg = QueryMsg<Lpns>;

pub(crate) struct Instantiator;

impl Instantiator {
    #[track_caller]
    pub fn instantiate_default<Lpn>(
        app: &mut App,
        lease_code: Code,
        init_balance: &[CwCoin],
        borrow_rate: InterestRate,
        min_utilization: Percent100,
    ) -> Addr
    where
        Lpn: CurrencyDef,
        Lpn::Group: MemberOf<Lpns>,
    {
        // TODO [Rust 1.70] Convert to static item with OnceCell
        let endpoints = CwContractWrapper::new(
            lpp::contract::execute,
            lpp::contract::instantiate,
            lpp::contract::query,
        )
        .with_sudo(sudo);

        Self::instantiate::<Lpn>(
            app,
            Box::new(endpoints),
            lease_code,
            init_balance,
            borrow_rate,
            min_utilization,
        )
    }

    #[track_caller]
    pub fn instantiate<Lpn>(
        app: &mut App,
        endpoints: Box<CwContract>,
        lease_code: Code,
        init_balance: &[CwCoin],
        borrow_rate: InterestRate,
        min_utilization: Percent100,
    ) -> Addr
    where
        Lpn: CurrencyDef,
        Lpn::Group: MemberOf<Lpns>,
    {
        let lpp_id = app.store_code(endpoints);
        let lease_code_admin = LeaserInstantiator::expected_addr();
        let msg = InstantiateMsg {
            lease_code_admin: lease_code_admin.clone(),
            lease_code: CodeId::from(lease_code).into(),
            borrow_rate,
            min_utilization,
        };

        app.instantiate(
            lpp_id,
            testing::user(ADMIN),
            &msg,
            init_balance,
            "lpp",
            None,
        )
        .unwrap()
        .unwrap_response()
    }
}

pub(crate) fn mock_quote_query(
    deps: Deps<'_>,
    env: Env,
    msg: QueryMsg<Lpns>,
) -> Result<Binary, ContractError> {
    let res = match msg {
        QueryMsg::Quote { amount: _amount } => to_json_binary(
            &lpp::msg::QueryQuoteResponse::QuoteInterestRate(Percent100::HUNDRED),
        ),
        _ => Ok(lpp::contract::query(deps, env, msg)?),
    }?;

    Ok(res)
}
