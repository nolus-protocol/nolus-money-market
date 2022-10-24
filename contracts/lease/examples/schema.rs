use lease::msg::{ExecuteMsg, NewLeaseForm, StateQuery};
use sdk::cosmwasm_schema::{export_schema, schema_for};

fn main() {
    let out_dir = schema::prep_out_dir().expect("The output directory should be valid");
    export_schema(&schema_for!(NewLeaseForm), &out_dir);
    export_schema(&schema_for!(ExecuteMsg), &out_dir);
    export_schema(&schema_for!(StateQuery), &out_dir);
}

#[cfg(test)]
#[test]
fn test_repay_representation() {
    use lease::msg::ExecuteMsg;
    use sdk::{
        cosmwasm_std::{from_slice, to_vec},
        schemars::_serde_json::to_string,
    };

    let msg = ExecuteMsg::Repay();
    let repay_bin = to_vec(&msg).expect("serialization failed");
    assert_eq!(msg, from_slice(&repay_bin).expect("deserialization failed"));

    assert_eq!(
        to_string(&msg).expect("deserialization failed"),
        r#"{"repay":[]}"#
    );
}
