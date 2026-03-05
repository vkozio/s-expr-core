use s_expr_core::{parse_with_dialect, DialectConfig, Limits, ParseErrorCode};

#[test]
fn enforces_maximum_nesting_depth() {
    let dialect = DialectConfig {
        limits: Limits {
            max_depth: Some(1),
            max_list_length: None,
            max_nodes: None,
        },
        ..DialectConfig::default()
    };

    let err = parse_with_dialect("(a (b))", &dialect).expect_err("expected depth limit violation");
    assert!(
        matches!(err.code, ParseErrorCode::LimitExceeded(_)),
        "unexpected error code: {:?}",
        err.code
    );
}

#[test]
fn enforces_maximum_list_length() {
    let dialect = DialectConfig {
        limits: Limits {
            max_depth: None,
            max_list_length: Some(1),
            max_nodes: None,
        },
        ..DialectConfig::default()
    };

    let err =
        parse_with_dialect("(a b)", &dialect).expect_err("expected list length limit violation");
    assert!(
        matches!(err.code, ParseErrorCode::LimitExceeded(_)),
        "unexpected error code: {:?}",
        err.code
    );
}

#[test]
fn enforces_maximum_node_count() {
    let dialect = DialectConfig {
        limits: Limits {
            max_depth: None,
            max_list_length: None,
            max_nodes: Some(1),
        },
        ..DialectConfig::default()
    };

    let err =
        parse_with_dialect("(a b)", &dialect).expect_err("expected node count limit violation");
    assert!(
        matches!(err.code, ParseErrorCode::LimitExceeded(_)),
        "unexpected error code: {:?}",
        err.code
    );
}
