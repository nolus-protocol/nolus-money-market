use std::collections::HashSet;

use finance::currency::SymbolOwned;
use finance::{liability::Liability, percent::Percent};
use lease::api::dex::ConnectionParams;
use lease::api::{DownpaymentCoin, InterestPaymentSpec};
use lpp::stub::lender::LppLenderRef;
use oracle::stub::OracleRef;
use sdk::{
    cosmwasm_ext::Response,
    cosmwasm_std::{Addr, Deps, DepsMut, MessageInfo, StdResult},
};

use crate::{
    cmd::Quote,
    error::{ContractError, ContractResult},
    msg::{ConfigResponse, QuoteResponse},
    state::config::Config,
    state::leaser::Loans,
};

pub struct Leaser {}

impl Leaser {
    pub fn query_config(deps: Deps) -> ContractResult<ConfigResponse> {
        let config = Config::load(deps.storage)?;
        Ok(ConfigResponse { config })
    }

    pub fn query_loans(deps: Deps, owner: Addr) -> StdResult<HashSet<Addr>> {
        Loans::get(deps.storage, owner)
    }

    pub fn query_quote(
        deps: Deps,
        downpayment: DownpaymentCoin,
        lease_asset: SymbolOwned,
    ) -> Result<QuoteResponse, ContractError> {
        let config = Config::load(deps.storage)?;

        let lpp = LppLenderRef::try_new(config.lpp_addr, &deps.querier, 0xDEADC0DEDEADC0DE)?;

        let oracle = OracleRef::try_from(config.market_price_oracle, &deps.querier)?;

        let resp = lpp.execute(
            Quote::new(
                deps.querier,
                downpayment,
                lease_asset,
                oracle,
                config.liability,
                config.lease_interest_rate_margin,
            )?,
            &deps.querier,
        )?;

        Ok(resp)
    }

    pub fn try_setup_dex(
        deps: DepsMut,
        info: MessageInfo,
        params: ConnectionParams,
    ) -> ContractResult<Response> {
        crate::access_control::OWNER
            .assert_address::<_, ContractError>(deps.as_ref(), &info.sender)?;

        Config::setup_dex(deps.storage, params)?;

        Ok(Response::default())
    }

    pub fn try_configure(
        deps: DepsMut,
        info: MessageInfo,
        lease_interest_rate_margin: Percent,
        liability: Liability,
        lease_interest_payment: InterestPaymentSpec,
    ) -> ContractResult<Response> {
        crate::access_control::OWNER
            .assert_address::<_, ContractError>(deps.as_ref(), &info.sender)?;

        Config::update(
            deps.storage,
            lease_interest_rate_margin,
            liability,
            lease_interest_payment,
        )?;

        Ok(Response::default())
    }
}
