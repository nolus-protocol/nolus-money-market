use std::env::current_dir;
use std::fs::create_dir_all;

use cosmwasm_schema::{export_schema, remove_schemas, schema_for};

use currency::native::Nls;
use lpp::msg::{
    BalanceResponse, ExecuteMsg, InstantiateMsg, LoanResponse, LppBalanceResponse, PriceResponse,
    QueryConfigResponse, QueryLoanOutstandingInterestResponse, QueryLoanResponse, QueryMsg,
    QueryQuoteResponse, RewardsResponse,
};

fn main() {
    let mut out_dir = current_dir().unwrap();
    out_dir.push("schema");
    create_dir_all(&out_dir).unwrap();
    remove_schemas(&out_dir).unwrap();

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
