use std::env::current_dir;
use std::fs::create_dir_all;

use cosmwasm_schema::{remove_schemas, schema_for, export_schema};
use lease::msg::{NewLeaseForm, ExecuteMsg, StatusQuery};

fn main() {
    let mut out_dir = current_dir().unwrap();
    out_dir.push("schema");
    create_dir_all(&out_dir).unwrap();
    remove_schemas(&out_dir).unwrap();

    export_schema(&schema_for!(NewLeaseForm), &out_dir);
    export_schema(&schema_for!(ExecuteMsg), &out_dir);
    export_schema(&schema_for!(StatusQuery), &out_dir);
}
