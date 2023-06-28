use currency::Currency;
use finance::{coin::Coin, duration::Duration, liability::Liability, percent::Percent, test};
use lease::api::InterestPaymentSpec;
use leaser::{
    contract::{execute, instantiate, query, reply, sudo},
    msg::{ExecuteMsg, InstantiateMsg, QueryMsg, QuoteResponse, SudoMsg},
    ContractError,
};
use sdk::{
    cosmwasm_std::{Addr, Uint64},
    cw_multi_test::Executor,
};

use crate::common::{ContractWrapper, MockApp};

use super::ADMIN;

pub struct LeaserWrapper {
    contract_wrapper: LeaserContractWrapperReply,
}
impl LeaserWrapper {
    pub const INTEREST_RATE_MARGIN: Percent = Percent::from_permille(30);

    pub const REPAYMENT_PERIOD: Duration = Duration::from_days(90);

    pub const GRACE_PERIOD: Duration = Duration::from_days(10);

    pub fn liability() -> Liability {
        Liability::new(
            Percent::from_percent(65),
            Percent::from_percent(5),
            Percent::from_percent(10),
            Percent::from_percent(2),
            Percent::from_percent(3),
            Percent::from_percent(2),
            Duration::from_hours(1),
        )
    }

    #[track_caller]
    pub fn instantiate(
        self,
        app: &mut MockApp,
        lease_code_id: u64,
        lpp_addr: &Addr,
        time_alarms: Addr,
        market_price_oracle: Addr,
        profit: Addr,
    ) -> Addr {
        let code_id = app.store_code(self.contract_wrapper);
        let msg = InstantiateMsg {
            lease_code_id: Uint64::new(lease_code_id),
            lpp_ust_addr: lpp_addr.clone(),
            lease_interest_rate_margin: Self::INTEREST_RATE_MARGIN,
            liability: Self::liability(),
            lease_interest_payment: InterestPaymentSpec::new(
                Self::REPAYMENT_PERIOD,
                Self::GRACE_PERIOD,
            ),
            time_alarms,
            market_price_oracle,
            profit,
        };

        app.instantiate_contract(code_id, Addr::unchecked(ADMIN), &msg, &[], "leaser", None)
            .unwrap()
    }
}

impl Default for LeaserWrapper {
    fn default() -> Self {
        let contract = ContractWrapper::new(execute, instantiate, query)
            .with_reply(reply)
            .with_sudo(sudo);

        Self {
            contract_wrapper: Box::new(contract),
        }
    }
}

type LeaserContractWrapperReply = Box<
    ContractWrapper<
        ExecuteMsg,
        ContractError,
        InstantiateMsg,
        ContractError,
        QueryMsg,
        ContractError,
        SudoMsg,
        ContractError,
        ContractError,
    >,
>;

pub fn query_quote<LpnC, DownpaymentC, LeaseC>(
    app: &mut MockApp,
    leaser: Addr,
    downpayment: Coin<DownpaymentC>,
) -> QuoteResponse
where
    LpnC: Currency,
    DownpaymentC: Currency,
    LeaseC: Currency,
{
    app.wrap()
        .query_wasm_smart(
            leaser,
            &QueryMsg::Quote {
                downpayment: test::funds::<_, DownpaymentC>(downpayment.into()),
                lease_asset: LeaseC::TICKER.into(),
                max_ltd: None,
            },
        )
        .unwrap()
}
