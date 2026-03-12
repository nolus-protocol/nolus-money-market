use std::marker::PhantomData;

use access_control::{AccessPermission, error::Error as AccessControlError, user::User};
use sdk::cosmwasm_std::{Addr, QuerierWrapper};
use serde::Serialize;

use crate::api::authz::AccessGranted;

pub struct RemotelyGrantedPermission<'authz_resolver, 'querier, QueryMsg, QueryMsgFactory> {
    authz_resolver: &'authz_resolver Addr,
    querier: QuerierWrapper<'querier>,
    _query_msg: PhantomData<QueryMsg>,
    query_msg_factory: QueryMsgFactory,
}

impl<'authz_resolver, 'querier, QueryMsg, QueryMsgFactory>
    RemotelyGrantedPermission<'authz_resolver, 'querier, QueryMsg, QueryMsgFactory>
{
    pub fn new(
        authz_resolver: &'authz_resolver Addr,
        querier: QuerierWrapper<'querier>,
        query_msg_factory: QueryMsgFactory,
    ) -> Self {
        Self {
            authz_resolver,
            querier,
            _query_msg: PhantomData,
            query_msg_factory,
        }
    }
}

impl<'authz_resolver, 'querier, QueryMsg, QueryMsgFactory> AccessPermission
    for RemotelyGrantedPermission<'authz_resolver, 'querier, QueryMsg, QueryMsgFactory>
where
    QueryMsg: Serialize,
    QueryMsgFactory: Fn(Addr) -> QueryMsg,
{
    fn granted_to<U>(&self, caller: &U) -> Result<bool, AccessControlError>
    where
        U: User,
    {
        let query = (self.query_msg_factory)(caller.addr().clone());
        self.querier
            .query_wasm_smart(self.authz_resolver, &query)
            .map_err(AccessControlError::Std)
            .map(|access: AccessGranted| access == AccessGranted::Yes)
    }
}
