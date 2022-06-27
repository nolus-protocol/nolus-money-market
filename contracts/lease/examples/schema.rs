use std::env::current_dir;
use std::fs::create_dir_all;

use cosmwasm_schema::{export_schema, remove_schemas, schema_for};
use lease::msg::{ExecuteMsg, NewLeaseForm, StateQuery};

fn main() {
    let mut out_dir = current_dir().unwrap();
    out_dir.push("schema");
    create_dir_all(&out_dir).unwrap();
    remove_schemas(&out_dir).unwrap();

    export_schema(&schema_for!(NewLeaseForm), &out_dir);
    export_schema(&schema_for!(ExecuteMsg), &out_dir);
    export_schema(&schema_for!(StateQuery), &out_dir);
}

#[cfg(test)]
#[test]
fn test_repay_representation() {
    use cosmwasm_std::{from_slice, to_vec};
    use lease::msg::ExecuteMsg;
    use schemars::_serde_json::to_string;

    let msg = ExecuteMsg::Repay();
    let repay_bin = to_vec(&msg).expect("serialization failed");
    assert_eq!(msg, from_slice(&repay_bin).expect("deserialization failed"));

    assert_eq!(
        r#"{"repay":[]}"#,
        to_string(&msg).expect("deserialization failed")
    );
}
