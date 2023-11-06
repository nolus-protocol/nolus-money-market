use std::collections::HashSet;

use currency::Currency;
use finance::{coin::Coin, duration::Duration, liability::Liability, percent::Percent, test};
use lease::api::{InterestPaymentSpec, PositionSpec};
use leaser::{
    contract::{execute, instantiate, query, reply, sudo},
    msg::{InstantiateMsg, QueryMsg, QuoteResponse},
};
use platform::contract::CodeId;
use sdk::cosmwasm_std::{Addr, Uint64};

use super::{test_case::app::App, CwContractWrapper, ADMIN};

pub(crate) struct Instantiator;

impl Instantiator {
    pub const INTEREST_RATE_MARGIN: Percent = Percent::from_permille(30);

    pub const REPAYMENT_PERIOD: Duration = Duration::from_days(90);

    pub const GRACE_PERIOD: Duration = Duration::from_days(10);

    pub const RECALC_TIME: Duration = Duration::from_hours(1);

    pub fn liability() -> Liability {
        Liability::new(
            Percent::from_percent(65),
            Percent::from_percent(5),
            Percent::from_percent(10),
            Percent::from_percent(2),
            Percent::from_percent(3),
            Percent::from_percent(2),
            Self::RECALC_TIME,
        )
    }

    pub fn position_spec() -> PositionSpec {
        PositionSpec::new(
            Self::liability(),
            super::lpn_coin(25_000_000),
            super::lpn_coin(5_000),
        )
    }

    #[track_caller]
    pub fn instantiate(
        app: &mut App,
        lease_code_id: CodeId,
        lpp_addr: Addr,
        time_alarms: Addr,
        market_price_oracle: Addr,
        profit: Addr,
    ) -> Addr {
        // TODO [Rust 1.70] Convert to static item with OnceCell
        let endpoints = CwContractWrapper::new(execute, instantiate, query)
            .with_reply(reply)
            .with_sudo(sudo);

        let code_id = app.store_code(Box::new(endpoints));

        let msg = InstantiateMsg {
            lease_code_id: Uint64::new(lease_code_id),
            lpp_ust_addr: lpp_addr,
            lease_interest_rate_margin: Self::INTEREST_RATE_MARGIN,
            lease_position_spec: Self::position_spec(),
            lease_interest_payment: InterestPaymentSpec::new(
                Self::REPAYMENT_PERIOD,
                Self::GRACE_PERIOD,
            ),
            time_alarms,
            market_price_oracle,
            profit,
        };

        app.instantiate(code_id, Addr::unchecked(ADMIN), &msg, &[], "leaser", None)
            .unwrap()
            .unwrap_response()
    }
}

pub(crate) fn query_quote<DownpaymentC, LeaseC>(
    app: &App,
    leaser: Addr,
    downpayment: Coin<DownpaymentC>,
    max_ltd: Option<Percent>,
) -> QuoteResponse
where
    DownpaymentC: Currency,
    LeaseC: Currency,
{
    app.query()
        .query_wasm_smart(
            leaser,
            &QueryMsg::Quote {
                downpayment: test::funds::<_, DownpaymentC>(downpayment.into()),
                lease_asset: LeaseC::TICKER.into(),
                max_ltd,
            },
        )
        .unwrap()
}

pub(crate) fn expect_a_lease(app: &App, leaser: Addr, customer: Addr) -> Addr {
    let leases = leases(app, leaser, customer);
    assert_eq!(1, leases.len());

    leases.into_iter().next().unwrap()
}

pub(crate) fn assert_no_leases(app: &App, leaser: Addr, customer: Addr) {
    assert!(leases(app, leaser, customer).is_empty());
}

pub(crate) fn assert_lease(app: &App, leaser: Addr, customer: Addr, lease: &Addr) {
    assert!(leases(app, leaser, customer).contains(lease));
}

fn leases(app: &App, leaser: Addr, customer: Addr) -> HashSet<Addr> {
    app.query()
        .query_wasm_smart(leaser, &QueryMsg::Leases { owner: customer })
        .unwrap()
}
