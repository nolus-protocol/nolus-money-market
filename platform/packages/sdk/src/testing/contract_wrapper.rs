use cosmwasm_std::{
    Binary, CustomMsg, CustomQuery, Deps, DepsMut, Empty, Env, MessageInfo, Reply, Response,
    StdError, StdResult, from_json,
};
use serde::de::DeserializeOwned;

use crate::cosmwasm_ext::InterChainMsg;

type ExecuteFnBox<C, Q> =
    Box<dyn for<'a> Fn(DepsMut<'a, Q>, Env, MessageInfo, Vec<u8>) -> StdResult<Response<C>>>;
type QueryFnBox<Q> = Box<dyn for<'a> Fn(Deps<'a, Q>, Env, Vec<u8>) -> StdResult<Binary>>;
type ReplyFnBox<C, Q> =
    Box<dyn for<'a> Fn(DepsMut<'a, Q>, Env, Reply) -> StdResult<Response<C>>>;
type SudoMigrateFnBox<C, Q> =
    Box<dyn for<'a> Fn(DepsMut<'a, Q>, Env, Vec<u8>) -> StdResult<Response<C>>>;

type ExecInstFnPtr<C, Q, T, E> =
    for<'a> fn(DepsMut<'a, Q>, Env, MessageInfo, T) -> Result<Response<C>, E>;
type ReplyFnPtr<C, Q, E> = for<'a> fn(DepsMut<'a, Q>, Env, Reply) -> Result<Response<C>, E>;
type SudoMigFnPtr<C, Q, T, E> = for<'a> fn(DepsMut<'a, Q>, Env, T) -> Result<Response<C>, E>;

/// A type-preserving contract wrapper.
///
/// Unlike `cw_multi_test::ContractWrapper`, this wrapper converts contract errors to `StdError`
/// using `Into<StdError>` (which preserves the original error type via `Box<dyn Error>`) instead
/// of `StdError::msg()` (which formats the error as a string, losing type information).
/// This allows `downcast_ref::<E>()` to work on the resulting `StdError`.
pub struct ContractWrapper<C = InterChainMsg, Q = Empty>
where
    Q: CustomQuery,
{
    execute_fn: ExecuteFnBox<C, Q>,
    instantiate_fn: ExecuteFnBox<C, Q>,
    query_fn: QueryFnBox<Q>,
    reply_fn: Option<ReplyFnBox<C, Q>>,
    sudo_fn: Option<SudoMigrateFnBox<C, Q>>,
    migrate_fn: Option<SudoMigrateFnBox<C, Q>>,
}

impl<C, Q> ContractWrapper<C, Q>
where
    C: CustomMsg + 'static,
    Q: CustomQuery + 'static,
{
    pub fn new<T1, T2, T3, E1, E2, E3>(
        execute: ExecInstFnPtr<C, Q, T1, E1>,
        instantiate: ExecInstFnPtr<C, Q, T2, E2>,
        query: fn(Deps<'_, Q>, Env, T3) -> Result<Binary, E3>,
    ) -> Self
    where
        T1: DeserializeOwned + 'static,
        T2: DeserializeOwned + 'static,
        T3: DeserializeOwned + 'static,
        E1: Into<StdError> + 'static,
        E2: Into<StdError> + 'static,
        E3: Into<StdError> + 'static,
    {
        Self {
            execute_fn: Box::new(move |deps, env, info, msg| {
                from_json::<T1>(msg)
                    .and_then(|msg| execute(deps, env, info, msg).map_err(Into::into))
            }),
            instantiate_fn: Box::new(move |deps, env, info, msg| {
                from_json::<T2>(msg)
                    .and_then(|msg| instantiate(deps, env, info, msg).map_err(Into::into))
            }),
            query_fn: Box::new(move |deps, env, msg| {
                from_json::<T3>(msg)
                    .and_then(|msg| query(deps, env, msg).map_err(Into::into))
            }),
            reply_fn: None,
            sudo_fn: None,
            migrate_fn: None,
        }
    }

    pub fn with_reply<E5>(
        mut self,
        reply: ReplyFnPtr<C, Q, E5>,
    ) -> Self
    where
        E5: Into<StdError> + 'static,
    {
        self.reply_fn = Some(Box::new(move |deps, env, msg| {
            reply(deps, env, msg).map_err(Into::into)
        }));
        self
    }

    pub fn with_sudo<T4, E4>(
        mut self,
        sudo: SudoMigFnPtr<C, Q, T4, E4>,
    ) -> Self
    where
        T4: DeserializeOwned + 'static,
        E4: Into<StdError> + 'static,
    {
        self.sudo_fn = Some(Box::new(move |deps, env, msg| {
            from_json::<T4>(msg).and_then(|msg| sudo(deps, env, msg).map_err(Into::into))
        }));
        self
    }

    pub fn with_migrate<T6, E6>(
        mut self,
        migrate: SudoMigFnPtr<C, Q, T6, E6>,
    ) -> Self
    where
        T6: DeserializeOwned + 'static,
        E6: Into<StdError> + 'static,
    {
        self.migrate_fn = Some(Box::new(move |deps, env, msg| {
            from_json::<T6>(msg).and_then(|msg| migrate(deps, env, msg).map_err(Into::into))
        }));
        self
    }
}

impl<C, Q> cw_multi_test::Contract<C, Q> for ContractWrapper<C, Q>
where
    C: CustomMsg,
    Q: CustomQuery,
{
    fn instantiate(
        &self,
        deps: DepsMut<'_, Q>,
        env: Env,
        info: MessageInfo,
        msg: Vec<u8>,
    ) -> StdResult<Response<C>> {
        (self.instantiate_fn)(deps, env, info, msg)
    }

    fn execute(
        &self,
        deps: DepsMut<'_, Q>,
        env: Env,
        info: MessageInfo,
        msg: Vec<u8>,
    ) -> StdResult<Response<C>> {
        (self.execute_fn)(deps, env, info, msg)
    }

    fn query(&self, deps: Deps<'_, Q>, env: Env, msg: Vec<u8>) -> StdResult<Binary> {
        (self.query_fn)(deps, env, msg)
    }

    fn reply(&self, deps: DepsMut<'_, Q>, env: Env, msg: Reply) -> StdResult<Response<C>> {
        self.reply_fn.as_ref().map_or_else(
            || Err(StdError::msg("reply is not implemented for contract")),
            |reply| reply(deps, env, msg),
        )
    }

    fn sudo(&self, deps: DepsMut<'_, Q>, env: Env, msg: Vec<u8>) -> StdResult<Response<C>> {
        self.sudo_fn.as_ref().map_or_else(
            || Err(StdError::msg("sudo is not implemented for contract")),
            |sudo| sudo(deps, env, msg),
        )
    }

    fn migrate(&self, deps: DepsMut<'_, Q>, env: Env, msg: Vec<u8>) -> StdResult<Response<C>> {
        self.migrate_fn.as_ref().map_or_else(
            || Err(StdError::msg("migrate is not implemented for contract")),
            |migrate| migrate(deps, env, msg),
        )
    }
}
