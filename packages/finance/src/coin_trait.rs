use cosmwasm_std::Coin as CosmWasmCoin;

pub trait Coin {}

struct NlsCoin;
impl Coin for NlsCoin {}

struct UsdcCoin;
impl Coin for UsdcCoin {}

pub fn from_cosmos(coin: CosmWasmCoin) -> Option<Box<dyn Coin>> {
    let _amount: u128 = coin.amount.into();
    let denom = coin.denom.as_str();
    if denom == "nls" {
        Some(Box::new(NlsCoin))
    } else if denom == "usdc" {
        Some(Box::new(UsdcCoin))
    } else {
        None
    }
}

#[cfg(test)]
mod test {
    use super::{from_cosmos, NlsCoin};
    use cosmwasm_std::Coin as CosmWasmCoin;

    #[test]
    fn test_from() {
        let pointer_coin = from_cosmos(CosmWasmCoin::new(12, "nls")).expect("nolus coin");
        // let coin: NlsCoin = *pointer_coin;
        // assert_eq!(Some(Box::new(NlsCoin)), );
    }
}
