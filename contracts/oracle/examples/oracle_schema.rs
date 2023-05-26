use oracle::{
    msg::{ConfigResponse, ExecuteMsg, InstantiateMsg, QueryMsg, SupportedCurrencyPairsResponse},
    state::config::Config,
};
use sdk::cosmwasm_schema::{export_schema, schema_for};
use versioning::SemVer;

fn main() {
    let out_dir = schema::prep_out_dir().expect("The output directory should be valid");

    export_schema(&schema_for!(InstantiateMsg), &out_dir);
    export_schema(&schema_for!(ExecuteMsg), &out_dir);
    export_schema(&schema_for!(QueryMsg), &out_dir);
    export_schema(&schema_for!(Config), &out_dir);
    export_schema(&schema_for!(ConfigResponse), &out_dir);
    export_schema(&schema_for!(SupportedCurrencyPairsResponse), &out_dir);
    export_schema(&schema_for!(SemVer), &out_dir);
}
