use topology::{Topology, error};

fn currency_definitions_error(source: &str, dex: &str) -> error::CurrencyDefinitions {
    serde_json::from_str::<'_, Topology>(source)
        .expect("Failed to deserialize testing JSON!")
        .currency_definitions(dex)
        .expect_err("Topology description should be invalid!")
}

#[test]
fn disconnected_host() {
    let error = currency_definitions_error(include_str!("disconnected_host.json"), "Dex");

    assert!(
        matches!(error, error::CurrencyDefinitions::HostNotConnectedToDex),
        "{error}"
    );
}

#[test]
fn disconnected_post_dex() {
    let error = currency_definitions_error(include_str!("disconnected_post_dex.json"), "Dex");

    let error::CurrencyDefinitions::ResolveCurrency(error::ResolveCurrency::NetworksNotConnected(
        from,
        to,
    )) = error
    else {
        panic!("{error}")
    };

    assert_eq!(&*from, "FarIntermediate2");

    assert_eq!(&*to, "FarIntermediate3");
}

#[test]
fn ibc_currency_cycle() {
    let error = currency_definitions_error(include_str!("ibc_currency_cycle.json"), "Dex");

    assert!(
        matches!(
            error,
            error::CurrencyDefinitions::ResolveCurrency(error::ResolveCurrency::CycleCreated)
        ),
        "{error}"
    );
}

#[test]
fn non_existent_currency() {
    let error = currency_definitions_error(include_str!("non_existent_currency_1.json"), "Dex");

    let error::CurrencyDefinitions::ResolveCurrency(error::ResolveCurrency::NoSuchCurrency(
        currency,
    )) = error
    else {
        panic!("{error}");
    };

    assert_eq!(&*currency, "HOST_NET_C_XYZ");

    let error = currency_definitions_error(include_str!("non_existent_currency_2.json"), "Dex");

    let error::CurrencyDefinitions::ResolveCurrency(error::ResolveCurrency::NoSuchCurrency(
        currency,
    )) = error
    else {
        panic!("{error}");
    };

    assert_eq!(&*currency, "FarC_XYZ");
}

#[test]
fn non_existent_network() {
    let error = currency_definitions_error(include_str!("non_existent_network_1.json"), "Dex");

    let error::CurrencyDefinitions::ResolveCurrency(error::ResolveCurrency::NoSuchNetwork(network)) =
        error
    else {
        panic!("{error}");
    };

    assert_eq!(&*network, "DexIntermediateXYZ");

    let error = currency_definitions_error(include_str!("non_existent_network_2.json"), "Dex");

    let error::CurrencyDefinitions::ResolveCurrency(error::ResolveCurrency::NoSuchNetwork(network)) =
        error
    else {
        panic!("{error}");
    };

    assert_eq!(&*network, "FarIntermediateXYZ");
}
