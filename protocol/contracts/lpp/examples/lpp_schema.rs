use currencies::Lpns;
use lpp::msg::{
    BalanceResponse, ExecuteMsg, InstantiateMsg, LoanResponse, PriceResponse, QueryLoanResponse,
    QueryMsg, QueryQuoteResponse, RewardsResponse,
};
use lpp_platform::{msg::StableBalanceResponse, Stable};
use sdk::cosmwasm_schema::{export_schema, schema_for};

fn main() {
    let out_dir = schema::prep_out_dir().expect("The output directory should be valid");

    export_schema(&schema_for!(InstantiateMsg), &out_dir);
    export_schema(&schema_for!(ExecuteMsg<Lpns>), &out_dir);
    export_schema(&schema_for!(QueryMsg<Lpns>), &out_dir);

    export_schema(&schema_for!(InstantiateMsg), &out_dir);
    export_schema(&schema_for!(QueryQuoteResponse), &out_dir);
    export_schema(&schema_for!(LoanResponse<Stable>), &out_dir);
    export_schema(&schema_for!(QueryLoanResponse<Stable>), &out_dir);
    export_schema(&schema_for!(BalanceResponse), &out_dir);
    export_schema(&schema_for!(PriceResponse<Stable>), &out_dir);
    export_schema(&schema_for!(StableBalanceResponse), &out_dir);
    export_schema(&schema_for!(RewardsResponse), &out_dir);
}
