use currency::native::Nls;
use lpp::msg::{
    BalanceResponse, ExecuteMsg, InstantiateMsg, LoanResponse, LppBalanceResponse, PriceResponse,
    QueryConfigResponse, QueryLoanOutstandingInterestResponse, QueryLoanResponse, QueryMsg,
    QueryQuoteResponse, RewardsResponse,
};
use sdk::cosmwasm_schema::{export_schema, schema_for};

fn main() {
    let out_dir = schema::prep_out_dir().expect("The output directory should be valid");

    export_schema(&schema_for!(InstantiateMsg), &out_dir);
    export_schema(&schema_for!(ExecuteMsg), &out_dir);
    export_schema(&schema_for!(QueryMsg), &out_dir);

    export_schema(&schema_for!(QueryConfigResponse), &out_dir);
    export_schema(&schema_for!(QueryQuoteResponse), &out_dir);
    export_schema(&schema_for!(LoanResponse<Nls>), &out_dir);
    export_schema(&schema_for!(QueryLoanResponse<Nls>), &out_dir);
    export_schema(
        &schema_for!(QueryLoanOutstandingInterestResponse<Nls>),
        &out_dir,
    );
    export_schema(&schema_for!(BalanceResponse), &out_dir);
    export_schema(&schema_for!(PriceResponse<Nls>), &out_dir);
    export_schema(&schema_for!(LppBalanceResponse<Nls>), &out_dir);
    export_schema(&schema_for!(RewardsResponse), &out_dir);
}
