use std::collections::BTreeMap;

use serde_test::{assert_tokens, Token};

use admin_contract::msg::{
    ContractsExecute, ExecuteSpec, Granularity, PlatformContracts, ProtocolContracts,
};
use json_value::JsonValue;

#[test]
fn contracts_execute() {
    const CONTRACTS_STRUCT_NAME: &str = "ContractsTemplate";
    const PLATFORM_FIELD_NAME: &str = "platform";
    const PROTOCOL_FIELD_NAME: &str = "protocol";
    const PLATFORM_STRUCT_NAME: &str = "PlatformContracts";
    const TIME_ALARMS_FIELD_NAME: &str = "timealarms";
    const TREASURY_FIELD_NAME: &str = "treasury";
    const PROTOCOL_STRUCT_NAME: &str = "ProtocolContracts";
    const GRANULARITY_ENUM_NAME: &str = "Granularity";
    const GRANULARITY_FIELD_NAME: &str = "some";

    const TIME_ALARMS_MESSAGE_FIELD_NAME: &str = "timealarms-config";
    const TIME_ALARMS_MESSAGE_FIELD_VALUE: i64 = 10;
    const TREASURY_MESSAGE_FIELD_NAME: &str = "treasury-config";
    const TREASURY_MESSAGE_FIELD_VALUE: i64 = 20;

    const SOME_PROTOCOL: &str = "1-some";
    const SOME_PROTOCOL_LEASER_MESSAGE_FIELD_NAME: &str = "some-leaser-config";
    const SOME_PROTOCOL_LEASER_MESSAGE_FIELD_VALUE: bool = false;
    const SOME_PROTOCOL_LPP_MESSAGE_FIELD_NAME: &str = "some-lpp-config";
    const SOME_PROTOCOL_ORACLE_MESSAGE_FIELD_NAME: &str = "some-oracle-config";
    const SOME_PROTOCOL_ORACLE_MESSAGE_FIELD_VALUE: bool = true;
    const SOME_PROTOCOL_PROFIT_MESSAGE_FIELD_NAME: &str = "some-profit-config";
    const SOME_PROTOCOL_PROFIT_MESSAGE_FIELD_VALUE: Vec<JsonValue> = vec![];
    const SOME_PROTOCOL_RESERVE_MESSAGE_FIELD_NAME: &str = "some-reserve-config";
    const SOME_PROTOCOL_RESERVE_MESSAGE_FIELD_VALUE: Vec<(String, JsonValue)> = vec![];

    const ALL_PROTOCOL: &str = "2-all";
    const ALL_PROTOCOL_LEASER_MESSAGE_FIELD_NAME: &str = "all-leaser-config";
    const ALL_PROTOCOL_LEASER_MESSAGE_FIELD_VALUE: i64 = 30;
    const ALL_PROTOCOL_LPP_MESSAGE_FIELD_NAME: &str = "all-lpp-config";
    const ALL_PROTOCOL_LPP_MESSAGE_FIELD_VALUE: i64 = 40;
    const ALL_PROTOCOL_ORACLE_MESSAGE_FIELD_NAME: &str = "all-oracle-config";
    const ALL_PROTOCOL_ORACLE_MESSAGE_FIELD_VALUE: i64 = 50;
    const ALL_PROTOCOL_PROFIT_MESSAGE_FIELD_NAME: &str = "all-profit-config";
    const ALL_PROTOCOL_PROFIT_MESSAGE_FIELD_VALUE: i64 = 60;
    const ALL_PROTOCOL_RESERVE_MESSAGE_FIELD_NAME: &str = "all-reserve-config";
    const ALL_PROTOCOL_RESERVE_MESSAGE_FIELD_VALUE: i64 = 70;

    const NULL_PROTOCOL: &str = "3-null";

    let value = ContractsExecute {
        platform: Granularity::All(Some(PlatformContracts {
            timealarms: ExecuteSpec {
                message: JsonValue::Object(vec![(
                    TIME_ALARMS_MESSAGE_FIELD_NAME.into(),
                    JsonValue::I64(TIME_ALARMS_MESSAGE_FIELD_VALUE),
                )]),
            },
            treasury: ExecuteSpec {
                message: JsonValue::Object(vec![(
                    TREASURY_MESSAGE_FIELD_NAME.into(),
                    JsonValue::I64(TREASURY_MESSAGE_FIELD_VALUE),
                )]),
            },
        })),
        protocol: BTreeMap::from([
            (
                SOME_PROTOCOL.into(),
                Granularity::Some {
                    some: ProtocolContracts {
                        leaser: Some(ExecuteSpec {
                            message: JsonValue::Object(vec![(
                                SOME_PROTOCOL_LEASER_MESSAGE_FIELD_NAME.into(),
                                JsonValue::Bool(SOME_PROTOCOL_LEASER_MESSAGE_FIELD_VALUE),
                            )]),
                        }),
                        lpp: None,
                        oracle: Some(ExecuteSpec {
                            message: JsonValue::Object(vec![(
                                SOME_PROTOCOL_ORACLE_MESSAGE_FIELD_NAME.into(),
                                JsonValue::Bool(SOME_PROTOCOL_ORACLE_MESSAGE_FIELD_VALUE),
                            )]),
                        }),
                        profit: Some(ExecuteSpec {
                            message: JsonValue::Object(vec![(
                                SOME_PROTOCOL_PROFIT_MESSAGE_FIELD_NAME.into(),
                                JsonValue::Array(SOME_PROTOCOL_PROFIT_MESSAGE_FIELD_VALUE),
                            )]),
                        }),
                        reserve: Some(ExecuteSpec {
                            message: JsonValue::Object(vec![(
                                SOME_PROTOCOL_RESERVE_MESSAGE_FIELD_NAME.into(),
                                JsonValue::Object(SOME_PROTOCOL_RESERVE_MESSAGE_FIELD_VALUE),
                            )]),
                        }),
                    },
                },
            ),
            (
                ALL_PROTOCOL.into(),
                Granularity::All(Some(ProtocolContracts {
                    leaser: ExecuteSpec {
                        message: JsonValue::Object(vec![(
                            ALL_PROTOCOL_LEASER_MESSAGE_FIELD_NAME.into(),
                            JsonValue::I64(ALL_PROTOCOL_LEASER_MESSAGE_FIELD_VALUE),
                        )]),
                    },
                    lpp: ExecuteSpec {
                        message: JsonValue::Object(vec![(
                            ALL_PROTOCOL_LPP_MESSAGE_FIELD_NAME.into(),
                            JsonValue::I64(ALL_PROTOCOL_LPP_MESSAGE_FIELD_VALUE),
                        )]),
                    },
                    oracle: ExecuteSpec {
                        message: JsonValue::Object(vec![(
                            ALL_PROTOCOL_ORACLE_MESSAGE_FIELD_NAME.into(),
                            JsonValue::I64(ALL_PROTOCOL_ORACLE_MESSAGE_FIELD_VALUE),
                        )]),
                    },
                    profit: ExecuteSpec {
                        message: JsonValue::Object(vec![(
                            ALL_PROTOCOL_PROFIT_MESSAGE_FIELD_NAME.into(),
                            JsonValue::I64(ALL_PROTOCOL_PROFIT_MESSAGE_FIELD_VALUE),
                        )]),
                    },
                    reserve: ExecuteSpec {
                        message: JsonValue::Object(vec![(
                            ALL_PROTOCOL_RESERVE_MESSAGE_FIELD_NAME.into(),
                            JsonValue::I64(ALL_PROTOCOL_RESERVE_MESSAGE_FIELD_VALUE),
                        )]),
                    },
                })),
            ),
            (NULL_PROTOCOL.into(), Granularity::All(None)),
        ]),
    };

    assert_tokens(
        &value,
        &r#struct(
            CONTRACTS_STRUCT_NAME,
            vec![
                (
                    PLATFORM_FIELD_NAME,
                    some(r#struct(
                        PLATFORM_STRUCT_NAME,
                        vec![
                            (
                                TIME_ALARMS_FIELD_NAME,
                                map(vec![(
                                    TIME_ALARMS_MESSAGE_FIELD_NAME.into(),
                                    i64(TIME_ALARMS_MESSAGE_FIELD_VALUE),
                                )]),
                            ),
                            (
                                TREASURY_FIELD_NAME,
                                map(vec![(
                                    TREASURY_MESSAGE_FIELD_NAME.into(),
                                    i64(TREASURY_MESSAGE_FIELD_VALUE),
                                )]),
                            ),
                        ],
                    )),
                ),
                (
                    PROTOCOL_FIELD_NAME,
                    map(vec![
                        (
                            SOME_PROTOCOL,
                            r#struct(
                                GRANULARITY_ENUM_NAME,
                                vec![(
                                    GRANULARITY_FIELD_NAME,
                                    r#struct(
                                        PROTOCOL_STRUCT_NAME,
                                        vec![
                                            (
                                                SOME_PROTOCOL_LEASER_MESSAGE_FIELD_NAME,
                                                some(bool(
                                                    SOME_PROTOCOL_LEASER_MESSAGE_FIELD_VALUE,
                                                )),
                                            ),
                                            (SOME_PROTOCOL_LPP_MESSAGE_FIELD_NAME, none()),
                                            (
                                                SOME_PROTOCOL_ORACLE_MESSAGE_FIELD_NAME,
                                                some(bool(
                                                    SOME_PROTOCOL_ORACLE_MESSAGE_FIELD_VALUE,
                                                )),
                                            ),
                                            (
                                                SOME_PROTOCOL_PROFIT_MESSAGE_FIELD_NAME,
                                                some(seq(vec![])),
                                            ),
                                            (
                                                SOME_PROTOCOL_RESERVE_MESSAGE_FIELD_NAME,
                                                some(map(vec![])),
                                            ),
                                        ],
                                    ),
                                )],
                            ),
                        ),
                        (
                            ALL_PROTOCOL,
                            some(r#struct(
                                PROTOCOL_STRUCT_NAME,
                                vec![
                                    (
                                        ALL_PROTOCOL_LEASER_MESSAGE_FIELD_NAME,
                                        i64(ALL_PROTOCOL_LEASER_MESSAGE_FIELD_VALUE),
                                    ),
                                    (
                                        ALL_PROTOCOL_LPP_MESSAGE_FIELD_NAME,
                                        i64(ALL_PROTOCOL_LPP_MESSAGE_FIELD_VALUE),
                                    ),
                                    (
                                        ALL_PROTOCOL_ORACLE_MESSAGE_FIELD_NAME,
                                        i64(ALL_PROTOCOL_ORACLE_MESSAGE_FIELD_VALUE),
                                    ),
                                    (
                                        ALL_PROTOCOL_PROFIT_MESSAGE_FIELD_NAME,
                                        i64(ALL_PROTOCOL_PROFIT_MESSAGE_FIELD_VALUE),
                                    ),
                                    (
                                        ALL_PROTOCOL_RESERVE_MESSAGE_FIELD_NAME,
                                        i64(ALL_PROTOCOL_RESERVE_MESSAGE_FIELD_VALUE),
                                    ),
                                ],
                            )),
                        ),
                        (NULL_PROTOCOL, none()),
                    ]),
                ),
            ],
        ),
    );

    assert_eq!(
        value,
        sdk::cosmwasm_std::from_json(sdk::cosmwasm_std::to_json_string(&value).unwrap()).unwrap()
    );
}

fn r#bool(x: bool) -> Vec<Token> {
    vec![Token::Bool(x)]
}

fn r#i64(x: i64) -> Vec<Token> {
    vec![Token::I64(x)]
}

fn none() -> Vec<Token> {
    vec![Token::None]
}

fn some(mut value: Vec<Token>) -> Vec<Token> {
    let mut v = vec![Token::Some];

    v.append(&mut value);

    v
}

fn seq(fields: Vec<Vec<Token>>) -> Vec<Token> {
    let mut v = vec![Token::Seq {
        len: Some(fields.len()),
    }];

    fields
        .into_iter()
        .for_each(|mut value| v.append(&mut value));

    v.push(Token::SeqEnd);

    v
}

fn r#struct(name: &'static str, fields: Vec<(&'static str, Vec<Token>)>) -> Vec<Token> {
    let mut v = vec![Token::Struct {
        name,
        len: fields.len(),
    }];

    fields
        .into_iter()
        .for_each(|(name, value)| v.append(&mut field(name, value)));

    v.push(Token::StructEnd);

    v
}

fn map(fields: Vec<(&'static str, Vec<Token>)>) -> Vec<Token> {
    let mut v = vec![Token::Map {
        len: Some(fields.len()),
    }];

    fields
        .into_iter()
        .for_each(|(name, value)| v.append(&mut field(name, value)));

    v.push(Token::MapEnd);

    v
}

fn field(name: &'static str, mut value: Vec<Token>) -> Vec<Token> {
    assert!(!value.is_empty());

    let mut v = vec![Token::Str(name)];

    v.append(&mut value);

    v
}
