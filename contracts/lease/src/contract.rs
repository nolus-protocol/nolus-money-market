use std::any;

#[cfg(feature = "cosmwasm-bindings")]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Reply, Response, StdResult, Storage, SubMsg,
};
use cw2::set_contract_version;
use finance::bank::{self, BankStub};
use finance::coin::Coin;
use finance::currency::{Currency, SymbolOwned, Usdc};
use lpp::stub::{Lpp, LppStub, LppVisitor};

use crate::error::{ContractError, ContractResult};
use crate::lease::Lease;
use crate::msg::{ExecuteMsg, NewLeaseForm, StateQuery, StateResponse};

const CONTRACT_NAME: &str = env!("CARGO_PKG_NAME");
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

type TheCurrency = Usdc;

#[cfg_attr(feature = "cosmwasm-bindings", entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: NewLeaseForm,
) -> ContractResult<Response> {
    // TODO restrict the Lease instantiation only to the Leaser addr by using `nolusd tx wasm store ... --instantiate-only-address <addr>`
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    // TODO query the market price oracle to get the price of the downpayment currency to LPN
    let downpayment = bank::received::<TheCurrency>(&info.funds)?;
    let lpp = lpp(&msg, &deps)?;

    // TODO 'receive' downpayment from the bank using Lpn
    // TODO store the lpp stub and load it on reply
    let req = lpp.execute(OpenLoanReq {
        form: &msg,
        downpayment,
    })?;
    msg.save(deps.storage)?;

    // TODO define an OpenLoanRequest(downpayment, borrowed_amount) and persist it

    Ok(Response::new().add_submessage(req))
}

#[cfg_attr(feature = "cosmwasm-bindings", entry_point)]
pub fn reply(deps: DepsMut, env: Env, msg: Reply) -> ContractResult<Response> {
    // TODO debug_assert the balance is increased with the borrowed amount
    // TODO load the top request and pass it as a reply
    // TODO swap the received loan and the downpayment to lease.currency
    let new_lease_form = NewLeaseForm::pull(deps.storage)?;
    let lpp = lpp(&new_lease_form, &deps)?;
    lpp.execute(OpenLoanResp { resp: msg })?;

    let lease: Lease<TheCurrency, _> = new_lease_form.into_lease(lpp, env.block.time, deps.api)?;
    lease.store(deps.storage)?;

    Ok(Response::default())
}

#[cfg_attr(feature = "cosmwasm-bindings", entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> ContractResult<Response> {
    match msg {
        ExecuteMsg::Repay() => try_repay(deps, env, info),
        ExecuteMsg::Close() => try_close(deps, env, info),
    }
}

#[cfg_attr(feature = "cosmwasm-bindings", entry_point)]
pub fn query(deps: Deps, env: Env, _msg: StateQuery) -> ContractResult<Binary> {
    let lease = load_lease(deps.storage)?;
    let bank_account = BankStub::my_account(&env, &deps.querier);
    let resp: StateResponse<TheCurrency, TheCurrency> = lease.state(
        env.block.time,
        &bank_account,
        &deps.querier,
        env.contract.address.clone(),
    )?;
    to_binary(&resp).map_err(ContractError::from)
}

fn try_repay(deps: DepsMut, env: Env, info: MessageInfo) -> ContractResult<Response> {
    let payment = bank::received::<TheCurrency>(&info.funds)?;
    let mut lease = load_lease(deps.storage)?;
    let lpp_loan_repay_req =
        lease.repay(payment, env.block.time, &deps.querier, env.contract.address)?;
    lease.store(deps.storage)?;
    let resp = if let Some(req) = lpp_loan_repay_req {
        Response::default().add_submessage(req)
    } else {
        Response::default()
    };
    Ok(resp)
}

fn try_close(deps: DepsMut, env: Env, info: MessageInfo) -> ContractResult<Response> {
    let lease = load_lease(deps.storage)?;
    if !lease.owned_by(&info.sender) {
        return ContractResult::Err(ContractError::Unauthorized {});
    }

    let bank_account = BankStub::my_account(&env, &deps.querier);
    let bank_req = lease.close(env.contract.address.clone(), &deps.querier, &bank_account)?;
    Ok(Response::default().add_submessage(bank_req))
}

fn lpp(form: &NewLeaseForm, deps: &DepsMut) -> StdResult<LppStub> {
    lpp::stub::LppStub::try_from(form.loan.lpp.clone(), deps.api, &deps.querier)
}

fn load_lease(storage: &dyn Storage) -> StdResult<Lease<TheCurrency, LppStub>> {
    Lease::load(storage)
}

struct OpenLoanReq<'a> {
    form: &'a NewLeaseForm,
    downpayment: Coin<TheCurrency>,
}
impl<'a> LppVisitor for OpenLoanReq<'a> {
    type Output = SubMsg;

    type Error = ContractError;

    fn on<C, L>(self, lpp: &L) -> Result<Self::Output, Self::Error>
    where
        L: Lpp<C>,
        C: Currency,
    {
        let downpayment_lpn: Coin<C> = swap::<TheCurrency, C>(self.downpayment);
        let borrow = self.form.amount_to_borrow(downpayment_lpn)?;
        
        lpp.open_loan_req(borrow).map_err(|e| e.into())
    }

    fn unknown_lpn(self, symbol: SymbolOwned) -> Result<Self::Output, Self::Error> {
        Err(ContractError::UnknownCurrency { symbol })
    }
}

fn swap<Cin, Cout>(coin_in: Coin<Cin>) -> Coin<Cout>
{
    // NB! `type_name` is not a proper way to compare currency types. Consider removing it once implement the real swap.
    assert_eq!(any::type_name::<Cin>(), any::type_name::<Cout>());
    Coin::<Cout>::new(coin_in.into())
}

struct OpenLoanResp {
    resp: Reply,
}

impl LppVisitor for OpenLoanResp {
    type Output = ();

    type Error = ContractError;

    fn on<C, L>(self, lpp: &L) -> Result<Self::Output, Self::Error>
    where
        L: Lpp<C>,
        C: Currency,
    {
        lpp.open_loan_resp(self.resp)
            .map_err(ContractError::OpenLoanError)
    }

    fn unknown_lpn(self, symbol: SymbolOwned) -> Result<Self::Output, Self::Error> {
        Err(ContractError::UnknownCurrency { symbol })
    }
}
