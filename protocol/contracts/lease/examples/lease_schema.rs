use lease::api::{open::NewLeaseForm, query::QueryMsg, ExecuteMsg};
use sdk::cosmwasm_schema::{export_schema, schema_for};

fn main() {
    let out_dir = schema::prep_out_dir().expect("The output directory should be valid");
    export_schema(&schema_for!(NewLeaseForm), &out_dir);
    export_schema(&schema_for!(ExecuteMsg), &out_dir);
    export_schema(&schema_for!(QueryMsg), &out_dir);
}
