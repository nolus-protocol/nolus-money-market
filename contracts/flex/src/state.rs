#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Laon {
    pub borower: Addr,
    pub collateral: Vec<Coin>,
    pub borrowed: Vec<Coin>,
}
