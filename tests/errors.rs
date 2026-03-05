use s_expr_core::{parse, ParseError, ParseErrorCode, Position, Span};

#[test]
fn unexpected_eof_error_variant_is_constructible() {
    let span = Span::new(Position::start(), Position::start());
    let err = ParseError::new(ParseErrorCode::UnexpectedEof, "unexpected eof", span);
    assert!(matches!(err.code, ParseErrorCode::UnexpectedEof));
    let rendered = err.to_string();
    assert!(
        rendered.contains("UnexpectedEof"),
        "error Display should include the code, got: {}",
        rendered
    );
}

#[test]
fn reports_unbalanced_paren() {
    let err = parse("(a b").expect_err("parse should fail on unbalanced list");
    assert!(matches!(err.code, ParseErrorCode::UnbalancedParen));
    assert!(err.span.start.byte < err.span.end.byte);
}

#[test]
fn reports_unterminated_string() {
    let err = parse("(\"foo").expect_err("parse should fail on unterminated string");
    assert!(matches!(err.code, ParseErrorCode::UnterminatedString));
    assert!(err.span.start.byte < err.span.end.byte);
}

#[test]
fn reports_invalid_string_escape_with_details() {
    let err = parse("(\"foo\\q\")").expect_err("parse should fail on invalid escape");
    assert!(matches!(err.code, ParseErrorCode::InvalidAtom));
    let details = err.details.expect("expected error details");
    assert!(details
        .expected
        .iter()
        .any(|s| s == "valid escape sequence"));
    assert_eq!(details.found.as_deref(), Some("q"));
}
