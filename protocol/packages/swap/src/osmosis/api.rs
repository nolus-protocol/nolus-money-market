use sdk::cosmos_sdk_proto::{cosmos::base::v1beta1::Coin as ProtoCoin, prost::Message};

pub(super) trait TypeUrl {
    const TYPE_URL: &str;
}

#[derive(PartialEq, Message)]
pub(super) struct MsgSwapExactAmountIn {
    #[prost(string, tag = "1")]
    pub sender: String,
    #[prost(message, repeated, tag = "2")]
    pub routes: Vec<SwapAmountInRoute>,
    #[prost(message, optional, tag = "3")]
    pub token_in: Option<ProtoCoin>,
    #[prost(string, tag = "4")]
    pub token_out_min_amount: String,
}

impl TypeUrl for MsgSwapExactAmountIn {
    const TYPE_URL: &str = "/osmosis.poolmanager.v1beta1.MsgSwapExactAmountIn";
}

#[derive(PartialEq, Eq, Message)]
pub(super) struct MsgSwapExactAmountInResponse {
    #[prost(string, tag = "1")]
    pub token_out_amount: String,
}

impl TypeUrl for MsgSwapExactAmountInResponse {
    const TYPE_URL: &str = "/osmosis.poolmanager.v1beta1.MsgSwapExactAmountInResponse";
}

#[derive(PartialEq, Eq, Message)]
pub(super) struct SwapAmountInRoute {
    #[prost(uint64, tag = "1")]
    pub pool_id: u64,
    #[prost(string, tag = "2")]
    pub token_out_denom: String,
}

impl TypeUrl for SwapAmountInRoute {
    const TYPE_URL: &str = "/osmosis.poolmanager.v1beta1.SwapAmountInRoute";
}
