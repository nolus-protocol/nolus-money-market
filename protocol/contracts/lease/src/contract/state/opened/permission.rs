use access_control::{AccessPermission, permissions::SingleUserPermission, user::User};
use currency::{Currency, Group, MemberOf};
use oracle_platform::OracleRef;

pub type ChangeClosePolicyPermission<'a> = SingleUserPermission<'a>;
pub type ClosePositionPermission<'a> = SingleUserPermission<'a>;

/// This is a permission given to deliver price alarms
pub struct PriceAlarmDelivery<'a, QuoteC, QuoteG>
where
    QuoteC: Currency + MemberOf<QuoteG>,
    QuoteG: Group,
{
    oracle_ref: &'a OracleRef<QuoteC, QuoteG>,
}

impl<'a, QuoteC, QuoteG> PriceAlarmDelivery<'a, QuoteC, QuoteG>
where
    QuoteC: Currency + MemberOf<QuoteG>,
    QuoteG: Group,
{
    pub fn new(oracle_ref: &'a OracleRef<QuoteC, QuoteG>) -> Self {
        Self { oracle_ref }
    }
}

impl<QuoteC, QuoteG> AccessPermission for PriceAlarmDelivery<'_, QuoteC, QuoteG>
where
    QuoteC: Currency + MemberOf<QuoteG>,
    QuoteG: Group,
{
    fn granted_to<U>(&self, user: &U) -> bool
    where
        U: User,
    {
        self.oracle_ref.owned_by(user.addr())
    }
}
