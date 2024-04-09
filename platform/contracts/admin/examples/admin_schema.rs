use admin_contract::msg::{
    ExecuteMsg, InstantiateMsg, MigrateMsg, PlatformQueryResponse, ProtocolQueryResponse,
    ProtocolsQueryResponse, QueryMsg, SudoMsg,
};
use sdk::cosmwasm_schema::{export_schema, schema_for};

fn main() {
    let out_dir = schema::prep_out_dir().expect("The output directory should be valid");

    export_schema(&schema_for!(InstantiateMsg), &out_dir);
    export_schema(&schema_for!(MigrateMsg), &out_dir);
    export_schema(&schema_for!(ExecuteMsg), &out_dir);
    export_schema(&schema_for!(SudoMsg), &out_dir);
    export_schema(&schema_for!(QueryMsg), &out_dir);
    export_schema(&schema_for!(PlatformQueryResponse), &out_dir);
    export_schema(&schema_for!(ProtocolsQueryResponse), &out_dir);
    export_schema(&schema_for!(ProtocolQueryResponse), &out_dir);
}
