use currencies::{
    LeaseGroup as AlarmCurrencies, Lpn as BaseCurrency, Lpns as BaseCurrencies,
    PaymentGroup as PriceCurrencies,
};
use oracle::api::{Config, ExecuteMsg, InstantiateMsg, QueryMsg};
use sdk::cosmwasm_schema::{export_schema, schema_for};
use versioning::SemVer;

fn main() {
    let out_dir = schema::prep_out_dir().expect("The output directory should be valid");

    export_schema(&schema_for!(InstantiateMsg::<PriceCurrencies>), &out_dir);
    export_schema(
        &schema_for!(ExecuteMsg::<BaseCurrency, BaseCurrencies, AlarmCurrencies, PriceCurrencies>),
        &out_dir,
    );
    export_schema(&schema_for!(QueryMsg::<PriceCurrencies>), &out_dir);
    export_schema(&schema_for!(Config), &out_dir);
    export_schema(&schema_for!(SemVer), &out_dir);
}
