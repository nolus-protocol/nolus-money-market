use serde::Deserialize;

use currency::{Group, SymbolOwned};

use crate::{coin::Amount, error::Error};

use super::CoinDTO as ValidatedDTO;

/// Brings invariant checking as a step in deserializing a CoinDTO
#[derive(Deserialize)]
pub(super) struct CoinDTO {
    amount: Amount,
    ticker: SymbolOwned,
}

impl<G> TryFrom<CoinDTO> for ValidatedDTO<G>
where
    G: Group,
{
    type Error = Error;

    fn try_from(dto: CoinDTO) -> Result<Self, Self::Error> {
        Self::new_checked(dto.amount, dto.ticker)
    }
}

#[cfg(test)]
mod test {
    use currency::test::{SubGroup, SubGroupTestC1, SuperGroup, SuperGroupTestC1};
    use sdk::cosmwasm_std;

    use crate::coin::{Coin, CoinDTO};

    #[test]
    fn deser_same_group() {
        let coin: CoinDTO<SuperGroup> = Coin::<SuperGroupTestC1>::new(4215).into();
        let coin_deser: CoinDTO<SuperGroup> = cosmwasm_std::to_json_vec(&coin)
            .and_then(cosmwasm_std::from_json)
            .expect("correct raw bytes");
        assert_eq!(coin, coin_deser);
    }

    #[test]
    fn deser_parent_group() {
        type CoinCurrency = SubGroupTestC1;
        type DirectGroup = SubGroup;
        type ParentGroup = SuperGroup;

        let coin: CoinDTO<DirectGroup> = Coin::<CoinCurrency>::new(4215).into();
        let coin_deser: CoinDTO<ParentGroup> = cosmwasm_std::to_json_vec(&coin)
            .and_then(cosmwasm_std::from_json)
            .expect("correct raw bytes");
        let coin_exp: CoinDTO<ParentGroup> = Coin::<CoinCurrency>::try_from(coin).unwrap().into();
        assert_eq!(coin_exp, coin_deser);
    }

    #[test]
    fn deser_wrong_group() {
        let coin: CoinDTO<SuperGroup> = Coin::<SuperGroupTestC1>::new(4215).into();
        let coin_raw = cosmwasm_std::to_json_vec(&coin).unwrap();

        assert!(cosmwasm_std::from_json::<CoinDTO<SubGroup>>(&coin_raw).is_err());
    }
}
