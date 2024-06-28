use std::{
    fmt::{Display, Formatter},
    marker::PhantomData,
};

use sdk::schemars::{self, JsonSchema};
use serde::{Deserialize, Serialize};

use crate::{
    group::MemberOf,
    never::{self, Never},
    AnyVisitor, AnyVisitorResult, Currency, Group, GroupVisit, SymbolOwned, SymbolStatic, Symbols,
    Tickers,
};

use crate::error::Error;

use super::CurrencyDTO as ValidatedDTO;

/// Brings invariant checking as a step in deserializing a CurrencyDTO
#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(transparent, deny_unknown_fields, rename_all = "snake_case")]
pub(super) struct CurrencyDTO(SymbolOwned);

impl Display for CurrencyDTO {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl<G> TryFrom<CurrencyDTO> for ValidatedDTO<G>
where
    G: Group,
{
    type Error = Error;

    fn try_from(dto: CurrencyDTO) -> Result<Self, Self::Error> {
        struct TypeToCurrency<G>(PhantomData<G>);
        impl<G> AnyVisitor for TypeToCurrency<G>
        where
            G: Group,
        {
            type VisitedG = G;
            type Output = ValidatedDTO<G>;

            type Error = Error; // TODO consider adding a non-falling visiting variant

            fn on<C>(self) -> AnyVisitorResult<Self>
            where
                C: Currency + MemberOf<G>,
            {
                Ok(ValidatedDTO::<G>::from_currency_type::<C>())
            }
        }
        Tickers::visit_any(&dto.0, TypeToCurrency(PhantomData))
    }
}

impl<G> From<ValidatedDTO<G>> for CurrencyDTO
where
    G: Group + MemberOf<G>,
{
    fn from(value: ValidatedDTO<G>) -> Self {
        #[derive(Debug)]
        struct TypeToTicker<G>(PhantomData<G>);
        impl<G> AnyVisitor for TypeToTicker<G> {
            type VisitedG = G;

            type Output = SymbolStatic;

            type Error = Never;

            fn on<C>(self) -> AnyVisitorResult<Self>
            where
                C: Symbols,
            {
                Ok(C::TICKER)
            }
        }

        let ticker = never::safe_unwrap(value.into_currency_type(TypeToTicker(PhantomData::<G>)));
        Self(ticker.into())
    }
}

#[cfg(test)]
mod test {
    use sdk::cosmwasm_std;

    use crate::test::{SubGroupCurrency, SubGroupTestC1, SuperGroupCurrency, SuperGroupTestC1};

    #[test]
    fn deser_same_group() {
        let coin: SuperGroupCurrency = SuperGroupCurrency::from_currency_type::<SuperGroupTestC1>();
        let coin_deser: SuperGroupCurrency = cosmwasm_std::to_json_vec(&coin)
            .and_then(cosmwasm_std::from_json)
            .expect("correct raw bytes");
        assert_eq!(coin, coin_deser);
    }

    #[test]
    fn deser_parent_group() {
        type CoinCurrency = SubGroupTestC1;
        type DirectGroup = SubGroupCurrency;
        type ParentGroup = SuperGroupCurrency;

        let coin = DirectGroup::from_currency_type::<CoinCurrency>();
        let coin_deser: ParentGroup = cosmwasm_std::to_json_vec(&coin)
            .and_then(cosmwasm_std::from_json)
            .expect("correct raw bytes");
        let coin_exp = ParentGroup::from_currency_type::<CoinCurrency>();
        assert_eq!(coin_exp, coin_deser);
    }

    #[test]
    fn deser_wrong_group() {
        let coin = SuperGroupCurrency::from_currency_type::<SuperGroupTestC1>();
        let coin_raw = cosmwasm_std::to_json_vec(&coin).unwrap();

        assert!(cosmwasm_std::from_json::<SubGroupCurrency>(&coin_raw).is_err());
    }
}
