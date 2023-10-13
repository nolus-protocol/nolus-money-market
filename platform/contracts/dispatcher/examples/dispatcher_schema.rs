use rewards_dispatcher::{
    msg::{ConfigResponse, ExecuteMsg, InstantiateMsg, QueryMsg, SudoMsg},
    state::DispatchLog,
};
use sdk::cosmwasm_schema::{export_schema, schema_for};

fn main() {
    let out_dir = schema::prep_out_dir().expect("The output directory should be valid");

    export_schema(&schema_for!(InstantiateMsg), &out_dir);
    export_schema(&schema_for!(ExecuteMsg), &out_dir);
    export_schema(&schema_for!(QueryMsg), &out_dir);
    export_schema(&schema_for!(SudoMsg), &out_dir);
    export_schema(&schema_for!(ConfigResponse), &out_dir);
    export_schema(&schema_for!(DispatchLog), &out_dir);
}
