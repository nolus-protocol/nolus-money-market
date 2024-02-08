use std::rc::Rc;

use topology::{
    Currency, CurrencyWithIcon, HostCurrency, HostNetwork, NativeCurrency, Network, Topology,
};

#[test]
fn against_snapshot() {
    const JSON: &str = include_str!("snapshot.json");

    let _: Topology<Rc<str>> = serde_json::from_str(JSON).unwrap();
}

#[test]
fn valid_no_channels() {
    const JSON: &str = r#"{
        "host_network": {
            "name": "HostChain",
            "currency": {
                "id": "Host",
                "native": {
                    "name": "HostToken",
                    "symbol": "microhost",
                    "decimal_digits": "6"
                }
            }
        },
        "networks": {
            "ChainA": {
                "currencies": {
                    "A": {
                        "native": {
                            "name": "AToken",
                            "symbol": "microa",
                            "decimal_digits": "6"
                        }
                    }
                }
            }
        },
        "channels": []
    }"#;

    let topology: Topology<String> = serde_json::from_str(JSON).unwrap();

    assert_eq!(
        topology,
        Topology {
            host_network: HostNetwork {
                name: "HostChain".into(),
                currency: HostCurrency {
                    id: "Host".into(),
                    native: NativeCurrency {
                        name: "HostToken".into(),
                        symbol: "microhost".into(),
                        decimal_digits: 6,
                    },
                },
            },
            networks: [(
                "ChainA".into(),
                Network {
                    currencies: [(
                        "A".into(),
                        CurrencyWithIcon {
                            currency: Currency::Native(NativeCurrency {
                                name: "AToken".into(),
                                symbol: "microa".into(),
                                decimal_digits: 6,
                            }),
                            icon: None,
                        },
                    ),]
                    .into(),
                    amm_pools: vec![],
                },
            ),]
            .into(),
            channels: [].into(),
        }
    );
}

#[test]
fn invalid_connected_to_self() {
    const JSON: &str = r#"{
        "host_network": {
            "name": "HostChain",
            "currency": {
                "id": "Host",
                "native": {
                    "name": "HostToken",
                    "symbol": "microhost",
                    "decimal_digits": "6"
                }
            }
        },
        "networks": {
            "ChainA": {
                "currencies": {
                    "A": {
                        "native": {
                            "name": "AToken",
                            "symbol": "microa",
                            "decimal_digits": "6"
                        }
                    }
                }
            }
        },
        "channels": [
            {
                "a": {
                    "network": "ChainA",
                    "channel": "channel-0"
                },
                "b": {
                    "network": "ChainA",
                    "channel": "channel-1"
                }
            }
        ]
    }"#;

    let json: serde_json::Value = serde_json::from_str(JSON).unwrap();

    let _ = serde_json::from_value::<Topology<String>>(json).unwrap_err();
}

#[test]
fn unknown_fields_in_currency() {
    const JSON: &str = r#"{
        "host_network": {
            "name": "HostChain",
            "currency": {
                "id": "Host",
                "native": {
                    "name": "HostToken",
                    "symbol": "microhost",
                    "decimal_digits": "6"
                }
            }
        },
        "networks": {
            "ChainA": {
                "currencies": {
                    "A": {
                        "native": {
                            "name": "AToken",
                            "symbol": "microa",
                            "decimal_digits": "6",
                            "unknown_field": null
                        }
                    }
                }
            }
        },
        "channels": []
    }"#;

    let json: serde_json::Value = serde_json::from_str(JSON).unwrap();

    let _ = serde_json::from_value::<Topology<String>>(json).unwrap_err();
}

#[test]
fn valid_disconnected_networks() {
    const JSON: &str = r#"{
        "host_network": {
            "name": "HostChain",
            "currency": {
                "id": "Host",
                "native": {
                    "name": "HostToken",
                    "symbol": "microhost",
                    "decimal_digits": "6"
                }
            }
        },
        "networks": {
            "ChainA": {
                "currencies": {
                    "A": {
                        "native": {
                            "name": "AToken",
                            "symbol": "microa",
                            "decimal_digits": "6"
                        }
                    }
                }
            },
            "ChainB": {
                "currencies": {
                    "B": {
                        "native": {
                            "name": "BToken",
                            "symbol": "microb",
                            "decimal_digits": "6"
                        }
                    }
                }
            }
        },
        "channels": []
    }"#;

    let topology: Topology<String> = serde_json::from_str(JSON).unwrap();

    assert_eq!(
        topology,
        Topology {
            host_network: HostNetwork {
                name: "HostChain".into(),
                currency: HostCurrency {
                    id: "Host".into(),
                    native: NativeCurrency {
                        name: "HostToken".into(),
                        symbol: "microhost".into(),
                        decimal_digits: 6,
                    },
                },
            },
            networks: [
                (
                    "ChainA".into(),
                    Network {
                        currencies: [(
                            "A".into(),
                            CurrencyWithIcon {
                                currency: Currency::Native(NativeCurrency {
                                    name: "AToken".into(),
                                    symbol: "microa".into(),
                                    decimal_digits: 6,
                                }),
                                icon: None,
                            },
                        ),]
                        .into(),
                        amm_pools: vec![],
                    },
                ),
                (
                    "ChainB".into(),
                    Network {
                        currencies: [(
                            "B".into(),
                            CurrencyWithIcon {
                                currency: Currency::Native(NativeCurrency {
                                    name: "BToken".into(),
                                    symbol: "microb".into(),
                                    decimal_digits: 6,
                                }),
                                icon: None,
                            },
                        ),]
                        .into(),
                        amm_pools: vec![],
                    },
                ),
            ]
            .into(),
            channels: [].into(),
        }
    );
}

#[test]
fn invalid_disconnected_host_foreign_network() {
    const JSON: &str = r#"{
        "host_network": {
            "name": "HostChain",
            "currency": {
                "id": "Host",
                "native": {
                    "name": "HostToken",
                    "symbol": "microhost",
                    "decimal_digits": "6"
                }
            }
        },
        "networks": {
            "ChainA": {
                "currencies": {
                    "A": {
                        "native": {
                            "name": "AToken",
                            "symbol": "microa",
                            "decimal_digits": "6"
                        }
                    },
                    "HostToken": {
                        "ibc": {
                            "network": "ChainA",
                            "currency": "A"
                        }
                    }
                }
            }
        },
        "channels": []
    }"#;

    let json: serde_json::Value = serde_json::from_str(JSON).unwrap();

    let _ = serde_json::from_value::<Topology<Rc<str>>>(json).unwrap_err();
}

#[test]
fn invalid_disconnected_foreign_networks() {
    const JSON: &str = r#"{
        "host_network": {
            "name": "HostChain",
            "currency": {
                "id": "Host",
                "native": {
                    "name": "HostToken",
                    "symbol": "microhost",
                    "decimal_digits": "6"
                }
            }
        },
        "networks": {
            "ChainA": {
                "currencies": {
                    "A": {
                        "native": {
                            "name": "AToken",
                            "symbol": "microa",
                            "decimal_digits": "6"
                        }
                    }
                }
            },
            "ChainB": {
                "currencies": {
                    "A": {
                        "ibc": {
                            "network": "ChainA",
                            "currency": "A"
                        }
                    }
                }
            }
        },
        "channels": []
    }"#;

    let json: serde_json::Value = serde_json::from_str(JSON).unwrap();

    let _ = serde_json::from_value::<Topology<Rc<str>>>(json).unwrap_err();
}

#[test]
fn valid_connected_networks() {
    type StrContainer = Rc<str>;

    const JSON: &str = r#"{
        "host_network": {
            "name": "HostChain",
            "currency": {
                "id": "Host",
                "native": {
                    "name": "HostToken",
                    "symbol": "microhost",
                    "decimal_digits": "6"
                }
            }
        },
        "networks": {
            "ChainA": {
                "currencies": {
                    "A": {
                        "native": {
                            "name": "AToken",
                            "symbol": "microa",
                            "decimal_digits": "6"
                        }
                    }
                }
            },
            "ChainB": {
                "currencies": {
                    "A": {
                        "ibc": {
                            "network": "ChainA",
                            "currency": "A"
                        }
                    }
                }
            }
        },
        "channels": [
            {
                "a": {
                    "network": "ChainA",
                    "ch": "channel-0"
                },
                "b": {
                    "network": "ChainB",
                    "ch": "channel-100"
                }
            }
        ]
    }"#;

    let topology: Topology<StrContainer> = serde_json::from_str(JSON).unwrap();

    assert_eq!(
        topology,
        Topology {
            host_network: HostNetwork {
                name: "HostChain".into(),
                currency: HostCurrency {
                    id: "Host".into(),
                    native: NativeCurrency {
                        name: "HostToken".into(),
                        symbol: "microhost".into(),
                        decimal_digits: 6,
                    },
                },
            },
            networks: [
                (
                    "ChainA".into(),
                    Network {
                        currencies: [(
                            "A".into(),
                            CurrencyWithIcon {
                                currency: Currency::Native(NativeCurrency {
                                    name: "AToken".into(),
                                    symbol: "microa".into(),
                                    decimal_digits: 6,
                                }),
                                icon: None,
                            },
                        ),]
                        .into(),
                        amm_pools: vec![],
                    },
                ),
                (
                    "ChainB".into(),
                    Network {
                        currencies: [(
                            "A".into(),
                            CurrencyWithIcon {
                                currency: Currency::Foreign {
                                    network: "ChainA".into(),
                                    currency: "A".into(),
                                },
                                icon: None,
                            },
                        ),]
                        .into(),
                        amm_pools: vec![],
                    },
                ),
            ]
            .into(),
            channels: [
                (
                    StrContainer::from("ChainA").into(),
                    [(
                        StrContainer::from("ChainB").into(),
                        StrContainer::from("channel-0").into(),
                    )]
                    .into(),
                ),
                (
                    StrContainer::from("ChainB").into(),
                    [(
                        StrContainer::from("ChainA").into(),
                        StrContainer::from("channel-100").into(),
                    )]
                    .into(),
                ),
            ]
            .into(),
        }
    );
}

#[test]
fn invalid_duplicated_channels() {
    type StrContainer = Rc<str>;

    const JSON: &str = r#"{
        "host_network": {
            "name": "HostChain",
            "currency": {
                "id": "Host",
                "native": {
                    "name": "HostToken",
                    "symbol": "microhost",
                    "decimal_digits": "6"
                }
            }
        },
        "networks": {
            "ChainA": {
                "currencies": {
                    "A": {
                        "native": {
                            "name": "AToken",
                            "symbol": "microa",
                            "decimal_digits": "6"
                        }
                    }
                }
            },
            "ChainB": {
                "currencies": {
                    "A": {
                        "ibc": {
                            "network": "ChainA",
                            "currency": "A"
                        }
                    }
                }
            }
        },
        "channels": [
            {
                "a": {
                    "network": "ChainA",
                    "ch": "channel-0"
                },
                "b": {
                    "network": "ChainB",
                    "ch": "channel-100"
                }
            },
            {
                "a": {
                    "network": "ChainA",
                    "ch": "channel-0"
                },
                "b": {
                    "network": "ChainB",
                    "ch": "channel-100"
                }
            }
        ]
    }"#;

    let json: serde_json::Value = serde_json::from_str(JSON).unwrap();

    let _ = serde_json::from_value::<Topology<StrContainer>>(json).unwrap_err();
}

#[test]
fn invalid_connected_to_undefined_network() {
    type StrContainer = Rc<str>;

    const JSON: &str = r#"{
        "host_network": {
            "name": "HostChain",
            "currency": {
                "id": "Host",
                "native": {
                    "name": "HostToken",
                    "symbol": "microhost",
                    "decimal_digits": "6"
                }
            }
        },
        "networks": {
            "ChainA": {
                "currencies": {
                    "A": {
                        "native": {
                            "name": "AToken",
                            "symbol": "microa",
                            "decimal_digits": "6"
                        }
                    }
                }
            },
            "ChainB": {
                "currencies": {
                    "A": {
                        "ibc": {
                            "network": "ChainA",
                            "currency": "A"
                        }
                    }
                }
            }
        },
        "channels": [
            {
                "a": {
                    "network": "ChainA",
                    "ch": "channel-0"
                },
                "b": {
                    "network": "ChainC",
                    "ch": "channel-100"
                }
            }
        ]
    }"#;

    let json: serde_json::Value = serde_json::from_str(JSON).unwrap();

    let _ = serde_json::from_value::<Topology<StrContainer>>(json).unwrap_err();
}

#[test]
fn invalid_currency_from_indirect_network() {
    type StrContainer = Rc<str>;

    const JSON: &str = r#"{
        "host_network": {
            "name": "HostChain",
            "currency": {
                "id": "Host",
                "native": {
                    "name": "HostToken",
                    "symbol": "microhost",
                    "decimal_digits": "6"
                }
            }
        },
        "networks": {
            "ChainA": {
                "currencies": {
                    "A": {
                        "native": {
                            "name": "AToken",
                            "symbol": "microa",
                            "decimal_digits": "6"
                        }
                    }
                }
            },
            "ChainB": {
                "currencies": {
                    "A": {
                        "ibc": {
                            "network": "ChainA",
                            "currency": "A"
                        }
                    }
                }
            },
            "ChainC": {
                "currencies": {
                    "A": {
                        "ibc": {
                            "network": "ChainA",
                            "currency": "A"
                        }
                    }
                }
            }
        },
        "channels": [
            {
                "a": {
                    "network": "ChainA",
                    "ch": "channel-0"
                },
                "b": {
                    "network": "ChainB",
                    "ch": "channel-100"
                }
            },
            {
                "a": {
                    "network": "ChainB",
                    "ch": "channel-101"
                },
                "b": {
                    "network": "ChainC",
                    "ch": "channel-1001"
                }
            }
        ]
    }"#;

    let json: serde_json::Value = serde_json::from_str(JSON).unwrap();

    let _ = serde_json::from_value::<Topology<StrContainer>>(json).unwrap_err();
}

#[test]
fn valid_currency_from_indirect_network() {
    type StrContainer = Rc<str>;

    const JSON: &str = r#"{
        "host_network": {
            "name": "HostChain",
            "currency": {
                "id": "Host",
                "native": {
                    "name": "HostToken",
                    "symbol": "microhost",
                    "decimal_digits": "6"
                }
            }
        },
        "networks": {
            "ChainA": {
                "currencies": {
                    "A": {
                        "native": {
                            "name": "AToken",
                            "symbol": "microa",
                            "decimal_digits": "6"
                        }
                    }
                }
            },
            "ChainB": {
                "currencies": {
                    "A": {
                        "ibc": {
                            "network": "ChainA",
                            "currency": "A"
                        }
                    }
                }
            },
            "ChainC": {
                "currencies": {
                    "A": {
                        "ibc": {
                            "network": "ChainB",
                            "currency": "A"
                        }
                    }
                }
            }
        },
        "channels": [
            {
                "a": {
                    "network": "ChainA",
                    "ch": "channel-0"
                },
                "b": {
                    "network": "ChainB",
                    "ch": "channel-100"
                }
            },
            {
                "a": {
                    "network": "ChainB",
                    "ch": "channel-101"
                },
                "b": {
                    "network": "ChainC",
                    "ch": "channel-1001"
                }
            }
        ]
    }"#;

    let topology: Topology<StrContainer> = serde_json::from_str(JSON).unwrap();

    assert_eq!(
        topology,
        Topology {
            host_network: HostNetwork {
                name: "HostChain".into(),
                currency: HostCurrency {
                    id: "Host".into(),
                    native: NativeCurrency {
                        name: "HostToken".into(),
                        symbol: "microhost".into(),
                        decimal_digits: 6,
                    },
                },
            },
            networks: [
                (
                    "ChainA".into(),
                    Network {
                        currencies: [(
                            "A".into(),
                            CurrencyWithIcon {
                                currency: Currency::Native(NativeCurrency {
                                    name: "AToken".into(),
                                    symbol: "microa".into(),
                                    decimal_digits: 6,
                                }),
                                icon: None,
                            },
                        ),]
                        .into(),
                        amm_pools: vec![],
                    },
                ),
                (
                    "ChainB".into(),
                    Network {
                        currencies: [(
                            "A".into(),
                            CurrencyWithIcon {
                                currency: Currency::Foreign {
                                    network: "ChainA".into(),
                                    currency: "A".into(),
                                },
                                icon: None,
                            },
                        ),]
                        .into(),
                        amm_pools: vec![],
                    },
                ),
                (
                    "ChainC".into(),
                    Network {
                        currencies: [(
                            "A".into(),
                            CurrencyWithIcon {
                                currency: Currency::Foreign {
                                    network: "ChainB".into(),
                                    currency: "A".into(),
                                },
                                icon: None,
                            },
                        ),]
                        .into(),
                        amm_pools: vec![],
                    },
                ),
            ]
            .into(),
            channels: [
                (
                    StrContainer::from("ChainA").into(),
                    [(
                        StrContainer::from("ChainB").into(),
                        StrContainer::from("channel-0").into(),
                    )]
                    .into()
                ),
                (
                    StrContainer::from("ChainB").into(),
                    [
                        (
                            StrContainer::from("ChainA").into(),
                            StrContainer::from("channel-100").into(),
                        ),
                        (
                            StrContainer::from("ChainC").into(),
                            StrContainer::from("channel-101").into(),
                        ),
                    ]
                    .into(),
                ),
                (
                    StrContainer::from("ChainC").into(),
                    [(
                        StrContainer::from("ChainB").into(),
                        StrContainer::from("channel-1001").into(),
                    )]
                    .into(),
                ),
            ]
            .into(),
        }
    );
}
