use topology::{CurrencyDefinition, Topology};

/// Returns iterator yielding [`CurrencyDefinition`]s, with the first one being
/// the host networks host currency.
fn currency_definitions_generator(
    source: &str,
    dex: &str,
) -> impl Iterator<Item = CurrencyDefinition> + use<> {
    let currency_definitions = serde_json::from_str::<'_, Topology>(source)
        .expect("Failed to deserialize testing JSON!")
        .currency_definitions(dex)
        .expect("Failed to create currency definitions!");

    IntoIterator::into_iter([currency_definitions.host_currency.into()])
        .chain(currency_definitions.dex_currencies)
}

#[track_caller]
fn expect<I: Iterator<Item = CurrencyDefinition>>(
    mut iter: I,
    ticker: &str,
    host_path: &str,
    host_symbol: &str,
    dex_path: &str,
    dex_symbol: &str,
    decimal_digits: u8,
) {
    let currency = iter
        .next()
        .expect("Expected at least one more currency definition!");

    assert_eq!(currency.ticker(), ticker, "{currency:?}");

    let host = currency.host();
    assert_eq!(host.path(), host_path, "{currency:?}");
    assert_eq!(host.symbol(), host_symbol, "{currency:?}");

    let dex = currency.dex();
    assert_eq!(dex.path(), dex_path, "{currency:?}");
    assert_eq!(dex.symbol(), dex_symbol, "{currency:?}");

    assert_eq!(currency.decimal_digits(), decimal_digits, "{currency:?}");
}

#[track_caller]
fn expect_end<I: Iterator<Item = CurrencyDefinition>>(mut iter: I) {
    assert_eq!(
        iter.next(),
        None,
        "Expected at least one more currency definition!"
    );
}

#[test]
fn snapshot() {
    let mut currencies = currency_definitions_generator(include_str!("snapshot.json"), "OSMOSIS");

    expect(
        &mut currencies,
        "NLS",
        "unls",
        "unls",
        "transfer/channel-1636/unls",
        "ibc/60CCD515066BDEC287A05074FC7157504D3D7FAC816DD59BBC8F4F84EAB226E6",
        6,
    );

    expect(
        &mut currencies,
        "AKT",
        "transfer/channel-0/transfer/channel-73/uakt",
        "ibc/1064EED4A8E99F9C1158680236D0C5C3EA6B8BB65C9F87DAC6BC759DD904D818",
        "transfer/channel-73/uakt",
        "ibc/7153C8C55DB988805FAC69E449B680A8BAAC15944B87CF210ADCD1A3A9542857",
        6,
    );

    expect(
        &mut currencies,
        "ATOM",
        "transfer/channel-0/transfer/channel-12/uatom",
        "ibc/ECFDE61B64BB920E087E7448C4C3FE356B7BD13A1C2153119E98816C964FE196",
        "transfer/channel-12/uatom",
        "ibc/A8C2D23A1E6F95DA4E48BA349667E322BD7A6C996D8A4AAE8BA72E190F3D1477",
        6,
    );

    expect(
        &mut currencies,
        "JUNO",
        "transfer/channel-0/transfer/channel-1/ujunox",
        "ibc/8FB044422997A8A77891DE729EC28638DDE4C81A54398F68149A058AA9B74D9F",
        "transfer/channel-1/ujunox",
        "ibc/8E2FEFCBD754FA3C97411F0126B9EC76191BAA1B3959CB73CECF396A4037BBF0",
        6,
    );

    expect(
        &mut currencies,
        "OSMO",
        "transfer/channel-0/uosmo",
        "ibc/ED07A3391A112B175915CD8FAF43A2DA8E4790EDE12566649D0C2F97716B8518",
        "uosmo",
        "uosmo",
        6,
    );

    expect(
        &mut currencies,
        "USDC_AXELAR",
        "transfer/channel-0/transfer/channel-3/uausdc",
        "ibc/5DE4FCAF68AE40F81F738C857C0D95F7C1BC47B00FA1026E85C1DD92524D4A11",
        "transfer/channel-3/uausdc",
        "ibc/6F34E1BD664C36CE49ACC28E60D62559A5F96C4F9A6CCE4FC5A67B2852E24CFE",
        6,
    );

    expect_end(currencies);
}

#[test]
fn with_intermediates() {
    let mut currencies =
        currency_definitions_generator(include_str!("with_intermediates.json"), "Dex");

    expect(
        &mut currencies,
        "HostC",
        "chostc",
        "chostc",
        "transfer/channel-10001/transfer/channel-1001/transfer/channel-101/\
        transfer/channel-11/chostc",
        "ibc/127DE8C2179188419C34E69BFF735D4D2D443C31F39272DF5970DAFFEF5CCBC0",
        2,
    );

    expect(
        &mut currencies,
        "DexC",
        "transfer/channel-0/transfer/channel-10/transfer/channel-100/transfer/\
         channel-1000/mdexc",
        "ibc/A6AA40138F66E74E258B593BBDDF99366E07070E602934450E0E8AC048533626",
        "mdexc",
        "mdexc",
        3,
    );

    expect(
        &mut currencies,
        "FarC",
        "transfer/channel-0/transfer/channel-10/transfer/channel-100/transfer/\
        channel-1000/transfer/channel-10000/transfer/channel-100000/transfer/\
        channel-1000000/transfer/channel-10000000/ufarc",
        "ibc/62A15D4E87CE7D72E59139CB75C6EA22D04FC098FE1178ADC96C70F3A27448BE",
        "transfer/channel-10000/transfer/channel-100000/transfer/channel-\
        1000000/transfer/channel-10000000/ufarc",
        "ibc/19CA222BFA498B666FC36E691BB9609466D1030EA773C3F33E4C2F4F5AA0916C",
        6,
    );

    expect_end(currencies);
}
