use super::*;

mod internal_error {
    use super::*;
    use crate::parser::ParserError;

    #[test]
    fn test_missing_eof() {
        let tokens = vec![JasmToken {
            kind: JasmTokenKind::Identifier("Test".to_string()),
            span: Span::new(0, 4),
        }];
        let err = JasmParser::parse(tokens).unwrap_err();
        assert!(matches!(err, ParserError::Internal(_)));
    }

    #[test]
    fn test_eof_isnt_the_last_token() {
        let tokens = vec![
            JasmToken {
                kind: JasmTokenKind::Eof,
                span: Span::new(0, 0),
            },
            JasmToken {
                kind: JasmTokenKind::Identifier("Test".to_string()),
                span: Span::new(0, 4),
            },
        ];
        let err = JasmParser::parse(tokens).unwrap_err();
        assert!(matches!(err, ParserError::Internal(_)));
    }
}
