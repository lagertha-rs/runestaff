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

mod class_directive_expected {
    use super::*;
    use crate::parser::ParserError;
    use rstest::rstest;

    #[rstest]
    #[case::dot_super(JasmTokenKind::DotSuper, Span::new(0, 6))]
    #[case::dot_method(JasmTokenKind::DotMethod, Span::new(0, 7))]
    #[case::dot_code(JasmTokenKind::DotCode, Span::new(0, 5))]
    #[case::dot_end(JasmTokenKind::DotEnd, Span::new(0, 4))]
    #[case::public(JasmTokenKind::Public, Span::new(0, 6))]
    #[case::static_kw(JasmTokenKind::Static, Span::new(0, 6))]
    #[case::identifier(JasmTokenKind::Identifier("HelloWorld".to_string()), Span::new(0, 10))]
    #[case::integer(JasmTokenKind::Integer(42), Span::new(0, 2))]
    #[case::string_literal(JasmTokenKind::StringLiteral("hello".to_string()), Span::new(0, 7))]
    #[case::open_paren(JasmTokenKind::OpenParen, Span::new(0, 1))]
    #[case::close_paren(JasmTokenKind::CloseParen, Span::new(0, 1))]
    #[case::open_bracket(JasmTokenKind::OpenBracket, Span::new(0, 1))]
    fn test_non_class_token_returns_error(#[case] token_kind: JasmTokenKind, #[case] span: Span) {
        let tokens = vec![
            JasmToken {
                kind: token_kind.clone(),
                span: span.clone(),
            },
            JasmToken {
                kind: JasmTokenKind::Eof,
                span: Span::new(100, 100),
            },
        ];

        let err = JasmParser::parse(tokens).unwrap_err();
        assert_eq!(err, ParserError::ClassDirectiveExpected(span, token_kind));
    }

    #[test]
    fn test_skips_leading_newlines_before_error() {
        let tokens = vec![
            JasmToken {
                kind: JasmTokenKind::Newline,
                span: Span::new(0, 1),
            },
            JasmToken {
                kind: JasmTokenKind::Newline,
                span: Span::new(1, 2),
            },
            JasmToken {
                kind: JasmTokenKind::Newline,
                span: Span::new(2, 3),
            },
            JasmToken {
                kind: JasmTokenKind::DotSuper,
                span: Span::new(3, 9),
            },
            JasmToken {
                kind: JasmTokenKind::Eof,
                span: Span::new(9, 9),
            },
        ];

        let err = JasmParser::parse(tokens).unwrap_err();
        assert_eq!(
            err,
            ParserError::ClassDirectiveExpected(Span::new(3, 9), JasmTokenKind::DotSuper)
        );
    }

    #[test]
    fn test_only_newlines_then_eof() {
        let tokens = vec![
            JasmToken {
                kind: JasmTokenKind::Newline,
                span: Span::new(0, 1),
            },
            JasmToken {
                kind: JasmTokenKind::Newline,
                span: Span::new(1, 2),
            },
            JasmToken {
                kind: JasmTokenKind::Eof,
                span: Span::new(2, 2),
            },
        ];

        let err = JasmParser::parse(tokens).unwrap_err();
        assert_eq!(err, ParserError::EmptyFile(Span::new(2, 2)));
    }

    #[test]
    fn test_eof_as_first_token() {
        let tokens = vec![JasmToken {
            kind: JasmTokenKind::Eof,
            span: Span::new(0, 0),
        }];

        let err = JasmParser::parse(tokens).unwrap_err();
        assert_eq!(err, ParserError::EmptyFile(Span::new(0, 0)));
    }

    #[test]
    fn test_single_newline_before_non_class_token() {
        let tokens = vec![
            JasmToken {
                kind: JasmTokenKind::Newline,
                span: Span::new(0, 1),
            },
            JasmToken {
                kind: JasmTokenKind::Public,
                span: Span::new(1, 7),
            },
            JasmToken {
                kind: JasmTokenKind::Eof,
                span: Span::new(7, 7),
            },
        ];

        let err = JasmParser::parse(tokens).unwrap_err();
        assert_eq!(
            err,
            ParserError::ClassDirectiveExpected(Span::new(1, 7), JasmTokenKind::Public)
        );
    }

    #[test]
    fn test_error_span_preserves_offset() {
        let tokens = vec![
            JasmToken {
                kind: JasmTokenKind::Identifier("main".to_string()),
                span: Span::new(42, 46),
            },
            JasmToken {
                kind: JasmTokenKind::Eof,
                span: Span::new(46, 46),
            },
        ];

        let err = JasmParser::parse(tokens).unwrap_err();
        assert_eq!(
            err,
            ParserError::ClassDirectiveExpected(
                Span::new(42, 46),
                JasmTokenKind::Identifier("main".to_string())
            )
        );
    }
}

mod parser_error_messages {
    use super::*;
    use crate::parser::ParserError;

    #[test]
    fn test_directive_error_message() {
        let err = ParserError::ClassDirectiveExpected(Span::new(0, 5), JasmTokenKind::DotCode);
        assert_eq!(err.message(), Some("unexpected directive".to_string()));
    }

    #[test]
    fn test_keyword_error_message() {
        let err = ParserError::ClassDirectiveExpected(Span::new(0, 6), JasmTokenKind::Public);
        assert_eq!(err.message(), Some("unexpected keyword".to_string()));
    }

    #[test]
    fn test_identifier_error_message() {
        let err = ParserError::ClassDirectiveExpected(
            Span::new(0, 4),
            JasmTokenKind::Identifier("main".to_string()),
        );
        assert_eq!(err.message(), Some("unexpected identifier".to_string()));
    }

    #[test]
    fn test_integer_error_message() {
        let err = ParserError::ClassDirectiveExpected(Span::new(0, 2), JasmTokenKind::Integer(42));
        assert_eq!(err.message(), Some("unexpected integer".to_string()));
    }

    #[test]
    fn test_string_literal_error_message() {
        let err = ParserError::ClassDirectiveExpected(
            Span::new(0, 7),
            JasmTokenKind::StringLiteral("hello".to_string()),
        );
        assert_eq!(err.message(), Some("unexpected string iteral".to_string()));
    }

    #[test]
    fn test_symbol_error_message() {
        let err = ParserError::ClassDirectiveExpected(Span::new(0, 1), JasmTokenKind::OpenParen);
        assert_eq!(err.message(), Some("unexpected symbol".to_string()));
    }

    #[test]
    fn test_empty_file_error_message() {
        let err = ParserError::EmptyFile(Span::new(0, 0));
        assert_eq!(err.message(), Some("empty file".to_string()));
    }

    #[test]
    fn test_empty_file_label() {
        let err = ParserError::EmptyFile(Span::new(0, 0));
        assert_eq!(
            err.label(),
            Some("The file contains no class definition.".to_string())
        );
    }

    #[test]
    fn test_empty_file_span() {
        let err = ParserError::EmptyFile(Span::new(5, 5));
        assert_eq!(err.as_range(), Some(5..5));
    }

    #[test]
    fn test_note_is_always_present() {
        let err = ParserError::ClassDirectiveExpected(Span::new(0, 5), JasmTokenKind::DotCode);
        assert_eq!(
            err.note(),
            Some("A Java assembly file must start with a '.class' definition.".to_string())
        );
    }

    #[test]
    fn test_note_is_present_for_empty_file() {
        let err = ParserError::EmptyFile(Span::new(0, 0));
        assert_eq!(
            err.note(),
            Some("A Java assembly file must start with a '.class' definition.".to_string())
        );
    }

    #[test]
    fn test_label_contains_token_display() {
        let err = ParserError::ClassDirectiveExpected(Span::new(0, 5), JasmTokenKind::DotCode);
        assert_eq!(
            err.label(),
            Some("The '.code' directive cannot appear before a class is defined.".to_string())
        );
    }

    #[test]
    fn test_label_for_identifier() {
        let err = ParserError::ClassDirectiveExpected(
            Span::new(0, 4),
            JasmTokenKind::Identifier("main".to_string()),
        );
        assert_eq!(
            err.label(),
            Some("The 'main' identifier cannot appear before a class is defined.".to_string())
        );
    }

    #[test]
    fn test_label_for_integer() {
        let err = ParserError::ClassDirectiveExpected(Span::new(0, 3), JasmTokenKind::Integer(123));
        assert_eq!(
            err.label(),
            Some("The '123' integer cannot appear before a class is defined.".to_string())
        );
    }

    #[test]
    fn test_label_for_symbol() {
        let err = ParserError::ClassDirectiveExpected(Span::new(0, 1), JasmTokenKind::CloseParen);
        assert_eq!(
            err.label(),
            Some("The ')' symbol cannot appear before a class is defined.".to_string())
        );
    }

    #[test]
    fn test_span_returns_correct_span() {
        let err = ParserError::ClassDirectiveExpected(Span::new(10, 15), JasmTokenKind::DotMethod);
        assert_eq!(err.as_range(), Some(10..15));
    }

    #[test]
    fn test_internal_error_message() {
        let err = ParserError::Internal("something broke".to_string());
        assert_eq!(
            err.message(),
            Some("Internal parser error: something broke".to_string())
        );
    }

    #[test]
    fn test_internal_error_has_no_note() {
        let err = ParserError::Internal("bug".to_string());
        assert_eq!(err.note(), None);
    }

    #[test]
    fn test_internal_error_has_no_label() {
        let err = ParserError::Internal("bug".to_string());
        assert_eq!(err.label(), None);
    }

    #[test]
    fn test_internal_error_has_no_range() {
        let err = ParserError::Internal("bug".to_string());
        assert_eq!(err.as_range(), None);
    }
}
