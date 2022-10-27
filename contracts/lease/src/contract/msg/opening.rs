use serde::Serialize;

use finance::{
    coin::{Amount, Coin},
    currency::Currency,
};
use lpp::stub::lender::{LppLender as LppLenderTrait, LppLenderRef};
use market_price_oracle::stub::{Oracle as OracleTrait, OracleRef};
use profit::stub::{Profit as ProfitTrait, ProfitRef};
use sdk::cosmwasm_std::{Addr, Api, QuerierWrapper, Timestamp};
use time_alarms::stub::{TimeAlarms as TimeAlarmsTrait, TimeAlarmsRef};

use crate::{
    error::{ContractError, ContractResult},
    lease::{self, IntoDTOResult, Lease, WithLeaseDeps},
    loan::Loan,
};

use serde::Deserialize;

use finance::{currency::SymbolOwned, duration::Duration, liability::Liability, percent::Percent};
use sdk::schemars::{self, JsonSchema};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct NewLeaseForm {
    /// The customer who wants to open a lease.
    pub customer: String,
    /// Ticker of the currency this lease will be about.
    pub currency: SymbolOwned,
    /// Liability parameters
    pub liability: Liability,
    pub loan: LoanForm,
    /// The time alarms contract the lease uses to get time notifications
    pub time_alarms: String,
    /// The oracle contract that sends market price alerts to the lease
    pub market_price_oracle: String,
    // /// Dex connection parameters
    // pub dex: ConnectionParams,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
#[serde(rename = "loan")]
/// The value remains intact.
pub struct LoanForm {
    /// The delta added on top of the LPP Loan interest rate.
    ///
    /// The amount, a part of any payment, goes to the Profit contract.
    pub annual_margin_interest: Percent,
    /// The Liquidity Provider Pool, LPP, that lends the necessary amount for this lease.
    pub lpp: String,
    /// How long is a period for which the interest is due
    pub interest_due_period: Duration,
    /// How long after the due period ends the interest may be paid before initiating a liquidation
    pub grace_period: Duration,
    /// The Profit contract to which the margin interest is sent.
    pub profit: String,
}

impl NewLeaseForm {
    pub(crate) fn into_lease(
        self,
        lease_addr: &Addr,
        start_at: Timestamp,
        // amount: &CoinDTO, TODO
        amount: Amount,
        api: &dyn Api,
        querier: &QuerierWrapper,
        deps: (LppLenderRef, OracleRef),
    ) -> ContractResult<IntoDTOResult> {
        // debug_assert_eq!(&self.currency, amount.symbol()); TODO
        debug_assert!(amount > 0);

        let profit = ProfitRef::try_from(api.addr_validate(&self.loan.profit)?, querier)?;
        let alarms = TimeAlarmsRef::try_from(api.addr_validate(&self.time_alarms)?, querier)?;

        lease::execute_deps(
            LeaseFactory {
                form: &self,
                lease_addr,
                start_at,
                amount,
                api,
            },
            &self.currency,
            deps.0,
            profit,
            alarms,
            deps.1,
            querier,
        )
    }
}

struct LeaseFactory<'a> {
    form: &'a NewLeaseForm,
    lease_addr: &'a Addr,
    start_at: Timestamp,
    // amount: &'a CoinDTO, TODO
    amount: Amount,
    api: &'a dyn Api,
}

impl<'a> WithLeaseDeps for LeaseFactory<'a> {
    type Output = IntoDTOResult;
    type Error = ContractError;

    fn exec<Lpn, Asset, Lpp, Profit, TimeAlarms, Oracle>(
        self,
        lpp: Lpp,
        profit: Profit,
        alarms: TimeAlarms,
        oracle: Oracle,
    ) -> Result<Self::Output, Self::Error>
    where
        Lpn: Currency + Serialize,
        Asset: Currency + Serialize,
        Lpp: LppLenderTrait<Lpn>,
        TimeAlarms: TimeAlarmsTrait,
        Oracle: OracleTrait<Lpn>,
        Profit: ProfitTrait,
    {
        let customer = self.api.addr_validate(&self.form.customer)?;
        let liability = self.form.liability;
        liability.invariant_held()?;

        let loan = Loan::new(
            self.start_at,
            lpp,
            self.form.loan.annual_margin_interest,
            self.form.loan.interest_due_period,
            self.form.loan.grace_period,
            profit,
        )?;
        let amount: Coin<Asset> = self.amount.into();

        Ok(Lease::<_, Asset, _, _, _, _>::new(
            self.lease_addr,
            customer,
            amount,
            self.start_at,
            liability,
            loan,
            (alarms, oracle),
        )?
        .into_dto())
    }
}

#[cfg(test)]
mod test {
    use std::any::type_name;

    use currency::{lease::Osmo, lpn::Usdc};
    use finance::error::Error as FinanceError;
    use finance::{currency::Currency, duration::Duration};
    use finance::{liability::Liability, percent::Percent};
    use lpp::stub::lender::LppLenderRef;
    use market_price_oracle::{msg::ConfigResponse as OracleConfigResponse, stub::OracleRef};
    use profit::msg::ConfigResponse as ProfitConfigResponse;
    use sdk::cosmwasm_std::{
        from_slice,
        testing::{MockApi, MockQuerier},
        to_binary, Addr, ContractInfoResponse, ContractResult, QuerierResult, QuerierWrapper,
        SystemResult, Timestamp, WasmQuery,
    };

    use crate::contract::msg::{LoanForm, NewLeaseForm};
    use crate::{error::ContractError, reply_id::ReplyId};

    const PROFIT_ADDR: &str = "f78wgdw";
    const ORACLE_ADDR: &str = "f383hddnslni";
    type Lpn = Usdc;

    #[test]
    fn amount_to_borrow_broken_invariant() {
        let lpp = "sdgg22d";
        let liability: Liability = from_slice(
            br#"{"initial":40,"healthy":50,"first_liq_warn":52,"second_liq_warn":53,"third_liq_warn":54,"max":54,"recalc_time":36000}"#,
        )
        .unwrap();
        assert!(liability.invariant_held().is_err());
        let lease = NewLeaseForm {
            customer: "ss1s1".into(),
            currency: Osmo::TICKER.into(),
            liability,
            loan: LoanForm {
                annual_margin_interest: Percent::from_percent(0),
                lpp: lpp.into(),
                interest_due_period: Duration::from_secs(100),
                grace_period: Duration::from_secs(10),
                profit: PROFIT_ADDR.into(),
            },
            time_alarms: "timealarms".into(),
            market_price_oracle: ORACLE_ADDR.into(),
        };
        let api = MockApi::default();

        let mut querier = MockQuerier::default();
        querier.update_wasm(config_req_handler);
        let err = lease
            .into_lease(
                &Addr::unchecked("test"),
                Timestamp::from_nanos(1000),
                10,
                &api,
                &QuerierWrapper::new(&querier),
                (
                    LppLenderRef::unchecked::<_, Lpn>(lpp, ReplyId::OpenLoanReq.into()),
                    OracleRef::unchecked::<_, Lpn>(ORACLE_ADDR),
                ),
            )
            .unwrap_err();

        assert_eq!(
            err,
            ContractError::from(FinanceError::BrokenInvariant(
                type_name::<Liability>().into(),
                "Third liquidation % should be < max %".into()
            ))
        );
    }

    fn config_req_handler(request: &WasmQuery) -> QuerierResult {
        let resp = match request {
            WasmQuery::Smart {
                contract_addr,
                msg: _,
            } => if contract_addr == PROFIT_ADDR {
                to_binary(&ProfitConfigResponse { cadence_hours: 2 })
            } else if contract_addr == ORACLE_ADDR {
                to_binary(&OracleConfigResponse {
                    base_asset: Lpn::TICKER.into(),
                    expected_feeders: Percent::from_percent(50),
                    owner: Addr::unchecked("3d3"),
                    price_feed_period: Duration::from_secs(12),
                })
            } else {
                unreachable!()
            }
            .unwrap(),
            WasmQuery::ContractInfo { contract_addr: _ } => {
                to_binary(&ContractInfoResponse::new(20, "creator")).unwrap()
            }
            &_ => unreachable!(),
        };
        SystemResult::Ok(ContractResult::Ok(resp))
    }
}
