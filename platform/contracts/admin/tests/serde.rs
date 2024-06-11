use std::collections::BTreeMap;

use serde_test::{assert_tokens, Token};

use admin_contract::msg::{ContractsExecute, Granularity, PlatformContracts, ProtocolContracts};

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
    const LEASER_FIELD_NAME: &str = "leaser";
    const LPP_FIELD_NAME: &str = "lpp";
    const ORACLE_FIELD_NAME: &str = "oracle";
    const PROFIT_FIELD_NAME: &str = "profit";
    const RESERVE_FIELD_NAME: &str = "reserve";

    const TIME_ALARMS_MSG: &str = r#"{"timealarms": 0}"#;
    const TREASURY_MSG: &str = r#"{"treasury": 0}"#;

    const SOME_PROTOCOL: &str = "1-some";
    const SOME_PROTOCOL_LEASER_MSG: &str = r#"{"leaser": 1}"#;
    const SOME_PROTOCOL_ORACLE_MSG: &str = r#"{"oracle": 1}"#;
    const SOME_PROTOCOL_PROFIT_MSG: &str = r#"{"profit": 1}"#;
    const SOME_PROTOCOL_RESERVE_MSG: &str = r#"{"reserve": 1}"#;

    const ALL_PROTOCOL: &str = "2-all";
    const ALL_PROTOCOL_LEASER_MSG: &str = r#"{"leaser": 2}"#;
    const ALL_PROTOCOL_LPP_MSG: &str = r#"{"lpp": 2}"#;
    const ALL_PROTOCOL_ORACLE_MSG: &str = r#"{"oracle": 2}"#;
    const ALL_PROTOCOL_PROFIT_MSG: &str = r#"{"profit": 2}"#;
    const ALL_PROTOCOL_RESERVE_MSG: &str = r#"{"reserve": 2}"#;

    const NULL_PROTOCOL: &str = "3-null";

    let value = ContractsExecute {
        platform: Granularity::All(Some(PlatformContracts {
            timealarms: TIME_ALARMS_MSG.into(),
            treasury: TREASURY_MSG.into(),
        })),
        protocol: BTreeMap::from([
            (
                SOME_PROTOCOL.into(),
                Granularity::Some {
                    some: ProtocolContracts {
                        leaser: Some(SOME_PROTOCOL_LEASER_MSG.into()),
                        lpp: None,
                        oracle: Some(SOME_PROTOCOL_ORACLE_MSG.into()),
                        profit: Some(SOME_PROTOCOL_PROFIT_MSG.into()),
                        reserve: Some(SOME_PROTOCOL_RESERVE_MSG.into()),
                    },
                },
            ),
            (
                ALL_PROTOCOL.into(),
                Granularity::All(Some(ProtocolContracts {
                    leaser: ALL_PROTOCOL_LEASER_MSG.into(),
                    lpp: ALL_PROTOCOL_LPP_MSG.into(),
                    oracle: ALL_PROTOCOL_ORACLE_MSG.into(),
                    profit: ALL_PROTOCOL_PROFIT_MSG.into(),
                    reserve: ALL_PROTOCOL_RESERVE_MSG.into(),
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
                            (TIME_ALARMS_FIELD_NAME, str(TIME_ALARMS_MSG)),
                            (TREASURY_FIELD_NAME, str(TREASURY_MSG)),
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
                                                LEASER_FIELD_NAME,
                                                some(str(SOME_PROTOCOL_LEASER_MSG)),
                                            ),
                                            (LPP_FIELD_NAME, none()),
                                            (
                                                ORACLE_FIELD_NAME,
                                                some(str(SOME_PROTOCOL_ORACLE_MSG)),
                                            ),
                                            (
                                                PROFIT_FIELD_NAME,
                                                some(str(SOME_PROTOCOL_PROFIT_MSG)),
                                            ),
                                            (
                                                RESERVE_FIELD_NAME,
                                                some(str(SOME_PROTOCOL_RESERVE_MSG)),
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
                                    (LEASER_FIELD_NAME, str(ALL_PROTOCOL_LEASER_MSG)),
                                    (LPP_FIELD_NAME, str(ALL_PROTOCOL_LPP_MSG)),
                                    (ORACLE_FIELD_NAME, str(ALL_PROTOCOL_ORACLE_MSG)),
                                    (PROFIT_FIELD_NAME, str(ALL_PROTOCOL_PROFIT_MSG)),
                                    (RESERVE_FIELD_NAME, str(ALL_PROTOCOL_RESERVE_MSG)),
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

fn none() -> Vec<Token> {
    vec![Token::None]
}

fn some(mut value: Vec<Token>) -> Vec<Token> {
    let mut v = vec![Token::Some];

    v.append(&mut value);

    v
}

fn r#str(s: &'static str) -> Vec<Token> {
    vec![Token::Str(s)]
}
