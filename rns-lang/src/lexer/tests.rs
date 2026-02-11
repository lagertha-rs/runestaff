use super::*;

mod snapshot_tests {
    use super::*;
    use insta::with_settings;
    use rstest::rstest;
    use std::io::Write;
    use std::path::{Path, PathBuf};
    use tabwriter::TabWriter;

    const SNAPSHOT_PATH: &str = "snapshots";

    fn format_tokens(tokens: &[JasmToken], source: &str) -> String {
        let mut tw = TabWriter::new(vec![]);

        // Header
        writeln!(tw, "KIND\t| SPAN\t| TEXT").unwrap();
        writeln!(tw, "----\t| ----\t| ----").unwrap();

        for token in tokens {
            let kind_str = match &token.kind {
                JasmTokenKind::Identifier(s) => format!("Identifier({:?})", s),
                JasmTokenKind::StringLiteral(s) => format!("StringLiteral({:?})", s),
                JasmTokenKind::Integer(n) => format!("Integer({})", n),
                other => format!("{:?}", other),
            };

            let span_str = format!("{}..{}", token.span.start, token.span.end);
            let text = &source[token.span.start..token.span.end];
            // Escape newlines and other control characters for display
            let text_display = text
                .replace('\n', "\\n")
                .replace('\r', "\\r")
                .replace('\t', "\\t");

            writeln!(tw, "{}\t| {}\t| {}", kind_str, span_str, text_display).unwrap();
        }

        tw.flush().unwrap();
        let tokens_table = String::from_utf8(tw.into_inner().unwrap()).unwrap();

        // Combine source and tokens table
        format!(
            "----- SOURCE -----\n{}\n----- TOKENS -----\n{}",
            source.trim_end(),
            tokens_table.trim_end()
        )
    }

    fn to_snapshot_name(path: &Path) -> String {
        let marker = Path::new("test_data/unit/lexer");
        let components = path.components().collect::<Vec<_>>();

        let marker_parts = marker.components().collect::<Vec<_>>();
        let idx = components
            .windows(marker_parts.len())
            .position(|window| window == marker_parts)
            .expect("Marker path not found in the given path");

        let after = &components[idx + marker_parts.len()..];
        let mut new_path = PathBuf::new();
        for c in after {
            new_path.push(c);
        }

        new_path.set_extension("");

        new_path
            .iter()
            .map(|s| s.to_string_lossy().to_string())
            .collect::<Vec<_>>()
            .join("-")
    }

    #[rstest]
    fn success_cases(
        #[base_dir = "test_data/unit/lexer/"]
        #[files("**/*.ja")]
        path: PathBuf,
    ) {
        let source = std::fs::read_to_string(&path).expect("Unable to read file");
        let mut lexer = JasmLexer::new(&source);
        let tokens = lexer
            .tokenize()
            .expect("Lexer should succeed for success test cases");

        let formatted = format_tokens(&tokens, &source);

        with_settings!(
            {
                snapshot_path => SNAPSHOT_PATH,
                prepend_module_to_snapshot => false,
            },
            {
                insta::assert_snapshot!(to_snapshot_name(&path), formatted);
            }
        );
    }
}

/*

//TODO: test unicode handling
mod directives {
    use super::*;
    use rstest::rstest;

    #[test]
    fn test_valid_tokenize_directives() {
        const INPUT: &str = ".class .super .method .end .code";
        let mut lexer = JasmLexer::new(INPUT);
        let tokens = lexer.tokenize().unwrap();

        assert_eq!(
            tokens,
            vec![
                JasmToken {
                    kind: JasmTokenKind::DotClass,
                    span: Span::new(0, 6),
                },
                JasmToken {
                    kind: JasmTokenKind::DotSuper,
                    span: Span::new(7, 13),
                },
                JasmToken {
                    kind: JasmTokenKind::DotMethod,
                    span: Span::new(14, 21),
                },
                JasmToken {
                    kind: JasmTokenKind::DotEnd,
                    span: Span::new(22, 26),
                },
                JasmToken {
                    kind: JasmTokenKind::DotCode,
                    span: Span::new(27, 32),
                },
                JasmToken {
                    kind: JasmTokenKind::Eof,
                    span: Span::new(32, 32),
                },
            ]
        )
    }

    #[test]
    fn test_valid_tokenize_on_diff_lines_directives() {
        const INPUT: &str = " \n    .class   .super \n .method  \n .end  \n ";
        let mut lexer = JasmLexer::new(INPUT);
        let tokens = lexer.tokenize().unwrap();

        assert_eq!(
            tokens,
            vec![
                JasmToken {
                    kind: JasmTokenKind::Newline,
                    span: Span::new(1, 2),
                },
                JasmToken {
                    kind: JasmTokenKind::DotClass,
                    span: Span::new(6, 12),
                },
                JasmToken {
                    kind: JasmTokenKind::DotSuper,
                    span: Span::new(15, 21),
                },
                JasmToken {
                    kind: JasmTokenKind::Newline,
                    span: Span::new(22, 23),
                },
                JasmToken {
                    kind: JasmTokenKind::DotMethod,
                    span: Span::new(24, 31),
                },
                JasmToken {
                    kind: JasmTokenKind::Newline,
                    span: Span::new(33, 34),
                },
                JasmToken {
                    kind: JasmTokenKind::DotEnd,
                    span: Span::new(35, 39),
                },
                JasmToken {
                    kind: JasmTokenKind::Newline,
                    span: Span::new(41, 42),
                },
                JasmToken {
                    kind: JasmTokenKind::Eof,
                    span: Span::new(43, 43),
                },
            ]
        )
    }

    #[rstest]
    #[case(".class\n    .unknown\n.method", 11, 19, ".unknown")]
    #[case(".super\n.;comment\n.unknown", 7, 16, ".;comment")]
    #[case(".super\n.;comment    \n.unknown", 7, 16, ".;comment")]
    #[case(".super\n.;comment ;ignored\n.unknown", 7, 16, ".;comment")]
    fn test_tokenize_unknown_directive(
        #[case] input: &str,
        #[case] start: usize,
        #[case] end: usize,
        #[case] name: &str,
    ) {
        let mut lexer = JasmLexer::new(input);
        let tokens = lexer.tokenize();

        assert_eq!(
            tokens,
            Err(LexerError::UnknownDirective(
                Span::new(start, end),
                name.to_string()
            ))
        )
    }

    #[test]
    fn test_tokenize_eof_directive() {
        const INPUT: &str = ".class\n    .";
        let mut lexer = JasmLexer::new(INPUT);
        let tokens = lexer.tokenize();

        assert_eq!(tokens, Err(LexerError::UnexpectedEof(Span::new(11, 12))))
    }

    #[rstest]
    #[case(".class\n    .\n.method", 12, '\n')]
    #[case(".class\n    . .code\n.method", 12, ' ')]
    #[case(".class\n    .\t.code\n.method", 12, '\t')]
    #[case(".class\n    .\r.code\n.method", 12, '\r')]
    fn test_tokenize_unexpected_char_directive(
        #[case] input: &str,
        #[case] pos: usize,
        #[case] c: char,
    ) {
        let mut lexer = JasmLexer::new(input);
        let tokens = lexer.tokenize();

        assert_eq!(
            tokens,
            Err(LexerError::UnexpectedChar(
                Span::new(pos, pos + c.len_utf8()),
                c,
                Some(format!(
                    "Expected one of the directives: {}",
                    JasmTokenKind::list_directives()
                )),
            ))
        )
    }
}

mod identifiers_and_keywords {
    use super::*;

    #[test]
    fn test_method_definition() {
        // With delimiter-based parsing, method signatures are tokenized into parts
        const INPUT: &str = ".method public static main([Ljava/lang/String;)V";
        let mut lexer = JasmLexer::new(INPUT);
        let tokens = lexer.tokenize().unwrap();

        assert_eq!(
            tokens,
            vec![
                JasmToken {
                    kind: JasmTokenKind::DotMethod,
                    span: Span::new(0, 7)
                },
                JasmToken {
                    kind: JasmTokenKind::Public,
                    span: Span::new(8, 14),
                },
                JasmToken {
                    kind: JasmTokenKind::Static,
                    span: Span::new(15, 21),
                },
                JasmToken {
                    kind: JasmTokenKind::Identifier("main".to_string()),
                    span: Span::new(22, 26),
                },
                JasmToken {
                    kind: JasmTokenKind::OpenParen,
                    span: Span::new(26, 27),
                },
                JasmToken {
                    kind: JasmTokenKind::OpenBracket,
                    span: Span::new(27, 28),
                },
                JasmToken {
                    kind: JasmTokenKind::Identifier("Ljava/lang/String;".to_string()),
                    span: Span::new(28, 46),
                },
                JasmToken {
                    kind: JasmTokenKind::CloseParen,
                    span: Span::new(46, 47),
                },
                JasmToken {
                    kind: JasmTokenKind::Identifier("V".to_string()),
                    span: Span::new(47, 48),
                },
                JasmToken {
                    kind: JasmTokenKind::Eof,
                    span: Span::new(48, 48),
                },
            ]
        )
    }

    #[test]
    fn test_method_definition_with_space_after_method_name() {
        const INPUT: &str = ".method public static main ([Ljava/lang/String;)V";
        let mut lexer = JasmLexer::new(INPUT);
        let tokens = lexer.tokenize().unwrap();

        assert_eq!(
            tokens,
            vec![
                JasmToken {
                    kind: JasmTokenKind::DotMethod,
                    span: Span::new(0, 7)
                },
                JasmToken {
                    kind: JasmTokenKind::Public,
                    span: Span::new(8, 14),
                },
                JasmToken {
                    kind: JasmTokenKind::Static,
                    span: Span::new(15, 21),
                },
                JasmToken {
                    kind: JasmTokenKind::Identifier("main".to_string()),
                    span: Span::new(22, 26),
                },
                JasmToken {
                    kind: JasmTokenKind::OpenParen,
                    span: Span::new(27, 28),
                },
                JasmToken {
                    kind: JasmTokenKind::OpenBracket,
                    span: Span::new(28, 29),
                },
                JasmToken {
                    kind: JasmTokenKind::Identifier("Ljava/lang/String;".to_string()),
                    span: Span::new(29, 47),
                },
                JasmToken {
                    kind: JasmTokenKind::CloseParen,
                    span: Span::new(47, 48),
                },
                JasmToken {
                    kind: JasmTokenKind::Identifier("V".to_string()),
                    span: Span::new(48, 49),
                },
                JasmToken {
                    kind: JasmTokenKind::Eof,
                    span: Span::new(49, 49),
                },
            ]
        )
    }

    #[test]
    fn test_tokenize_identifiers_and_keywords() {
        const INPUT: &str = "public static myVar another_var _privateVar";
        let mut lexer = JasmLexer::new(INPUT);
        let tokens = lexer.tokenize().unwrap();

        assert_eq!(
            tokens,
            vec![
                JasmToken {
                    kind: JasmTokenKind::Public,
                    span: Span::new(0, 6),
                },
                JasmToken {
                    kind: JasmTokenKind::Static,
                    span: Span::new(7, 13),
                },
                JasmToken {
                    kind: JasmTokenKind::Identifier("myVar".to_string()),
                    span: Span::new(14, 19),
                },
                JasmToken {
                    kind: JasmTokenKind::Identifier("another_var".to_string()),
                    span: Span::new(20, 31),
                },
                JasmToken {
                    kind: JasmTokenKind::Identifier("_privateVar".to_string()),
                    span: Span::new(32, 43),
                },
                JasmToken {
                    kind: JasmTokenKind::Eof,
                    span: Span::new(43, 43),
                },
            ]
        )
    }
}

mod strings {
    use super::*;
    use rstest::rstest;

    mod basic_parsing {
        use super::*;

        #[test]
        fn test_simple_string() {
            let mut lexer = JasmLexer::new(r#""hello""#);
            let tokens = lexer.tokenize().unwrap();

            assert_eq!(
                tokens,
                vec![
                    JasmToken {
                        kind: JasmTokenKind::StringLiteral("hello".to_string()),
                        span: Span::new(0, 7),
                    },
                    JasmToken {
                        kind: JasmTokenKind::Eof,
                        span: Span::new(7, 7),
                    },
                ]
            );
        }

        #[test]
        fn test_empty_string() {
            let mut lexer = JasmLexer::new(r#""""#);
            let tokens = lexer.tokenize().unwrap();

            assert_eq!(
                tokens,
                vec![
                    JasmToken {
                        kind: JasmTokenKind::StringLiteral("".to_string()),
                        span: Span::new(0, 2),
                    },
                    JasmToken {
                        kind: JasmTokenKind::Eof,
                        span: Span::new(2, 2),
                    },
                ]
            );
        }

        #[test]
        fn test_string_with_spaces() {
            let mut lexer = JasmLexer::new(r#""hello world""#);
            let tokens = lexer.tokenize().unwrap();

            assert_eq!(
                tokens,
                vec![
                    JasmToken {
                        kind: JasmTokenKind::StringLiteral("hello world".to_string()),
                        span: Span::new(0, 13),
                    },
                    JasmToken {
                        kind: JasmTokenKind::Eof,
                        span: Span::new(13, 13),
                    },
                ]
            );
        }

        #[test]
        fn test_string_with_numbers_and_special_chars() {
            let mut lexer = JasmLexer::new(r#""abc123!@#$%^&*()""#);
            let tokens = lexer.tokenize().unwrap();

            assert_eq!(
                tokens,
                vec![
                    JasmToken {
                        kind: JasmTokenKind::StringLiteral("abc123!@#$%^&*()".to_string()),
                        span: Span::new(0, 18),
                    },
                    JasmToken {
                        kind: JasmTokenKind::Eof,
                        span: Span::new(18, 18),
                    },
                ]
            );
        }
    }

    mod escape_sequences {
        use super::*;

        #[rstest]
        #[case(r#""hello\nworld""#, "hello\nworld", 14)]
        #[case(r#""tab\there""#, "tab\there", 11)]
        #[case(r#""line\rbreak""#, "line\rbreak", 13)]
        #[case(r#""null\0char""#, "null\0char", 12)]
        #[case(r#""back\bspace""#, "back\x08space", 13)]
        #[case(r#""form\ffeed""#, "form\x0Cfeed", 12)]
        #[case(r#""say \"hello\"""#, "say \"hello\"", 15)]
        #[case(r#""back\\slash""#, "back\\slash", 13)]
        fn test_escape_sequence(
            #[case] input: &str,
            #[case] expected: &str,
            #[case] end_pos: usize,
        ) {
            let mut lexer = JasmLexer::new(input);
            let tokens = lexer.tokenize().unwrap();

            assert_eq!(
                tokens,
                vec![
                    JasmToken {
                        kind: JasmTokenKind::StringLiteral(expected.to_string()),
                        span: Span::new(0, end_pos),
                    },
                    JasmToken {
                        kind: JasmTokenKind::Eof,
                        span: Span::new(end_pos, end_pos),
                    },
                ]
            );
        }

        #[test]
        fn test_unknown_escape_passes_through() {
            // Unknown escape sequences like \x should pass through the character
            let mut lexer = JasmLexer::new(r#""unknown\xescape""#);
            let tokens = lexer.tokenize().unwrap();

            assert_eq!(
                tokens,
                vec![
                    JasmToken {
                        kind: JasmTokenKind::StringLiteral("unknownxescape".to_string()),
                        span: Span::new(0, 17),
                    },
                    JasmToken {
                        kind: JasmTokenKind::Eof,
                        span: Span::new(17, 17),
                    },
                ]
            );
        }

        #[test]
        fn test_multiple_escapes_in_one_string() {
            let mut lexer = JasmLexer::new(r#""line1\nline2\ttab\r\n""#);
            let tokens = lexer.tokenize().unwrap();

            assert_eq!(
                tokens,
                vec![
                    JasmToken {
                        kind: JasmTokenKind::StringLiteral("line1\nline2\ttab\r\n".to_string()),
                        span: Span::new(0, 23),
                    },
                    JasmToken {
                        kind: JasmTokenKind::Eof,
                        span: Span::new(23, 23),
                    },
                ]
            );
        }

        #[test]
        fn test_escaped_quote_at_string_boundaries() {
            // String that starts and ends with escaped quotes
            let mut lexer = JasmLexer::new(r#""\"hello\"""#);
            let tokens = lexer.tokenize().unwrap();

            assert_eq!(
                tokens,
                vec![
                    JasmToken {
                        kind: JasmTokenKind::StringLiteral("\"hello\"".to_string()),
                        span: Span::new(0, 11),
                    },
                    JasmToken {
                        kind: JasmTokenKind::Eof,
                        span: Span::new(11, 11),
                    },
                ]
            );
        }

        #[test]
        fn test_consecutive_backslashes() {
            // "\\\\" should become "\\"
            let mut lexer = JasmLexer::new(r#""\\\\""#);
            let tokens = lexer.tokenize().unwrap();

            assert_eq!(
                tokens,
                vec![
                    JasmToken {
                        kind: JasmTokenKind::StringLiteral("\\\\".to_string()),
                        span: Span::new(0, 6),
                    },
                    JasmToken {
                        kind: JasmTokenKind::Eof,
                        span: Span::new(6, 6),
                    },
                ]
            );
        }

        #[test]
        fn test_backslash_followed_by_literal_newline() {
            // String with backslash followed by actual newline should be treated as unterminated
            let mut lexer = JasmLexer::new("\"test\\\n");
            let result = lexer.tokenize();

            assert_eq!(result, Err(LexerError::UnterminatedString(Span::new(0, 1))));
        }

        #[test]
        fn test_backslash_followed_by_literal_carriage_return() {
            // String with backslash followed by carriage return should be treated as unterminated
            let mut lexer = JasmLexer::new("\"test\\\r");
            let result = lexer.tokenize();

            assert_eq!(result, Err(LexerError::UnterminatedString(Span::new(0, 1))));
        }
    }

    mod edge_cases {
        use super::*;

        #[test]
        fn test_multiple_consecutive_strings() {
            let mut lexer = JasmLexer::new(r#""str1" "str2""#);
            let tokens = lexer.tokenize().unwrap();

            assert_eq!(
                tokens,
                vec![
                    JasmToken {
                        kind: JasmTokenKind::StringLiteral("str1".to_string()),
                        span: Span::new(0, 6),
                    },
                    JasmToken {
                        kind: JasmTokenKind::StringLiteral("str2".to_string()),
                        span: Span::new(7, 13),
                    },
                    JasmToken {
                        kind: JasmTokenKind::Eof,
                        span: Span::new(13, 13),
                    },
                ]
            );
        }

        #[test]
        fn test_string_followed_by_identifier() {
            let mut lexer = JasmLexer::new(r#""hello" world"#);
            let tokens = lexer.tokenize().unwrap();

            assert_eq!(
                tokens,
                vec![
                    JasmToken {
                        kind: JasmTokenKind::StringLiteral("hello".to_string()),
                        span: Span::new(0, 7),
                    },
                    JasmToken {
                        kind: JasmTokenKind::Identifier("world".to_string()),
                        span: Span::new(8, 13),
                    },
                    JasmToken {
                        kind: JasmTokenKind::Eof,
                        span: Span::new(13, 13),
                    },
                ]
            );
        }

        #[test]
        fn test_string_followed_by_keyword() {
            let mut lexer = JasmLexer::new(r#""test" public"#);
            let tokens = lexer.tokenize().unwrap();

            assert_eq!(
                tokens,
                vec![
                    JasmToken {
                        kind: JasmTokenKind::StringLiteral("test".to_string()),
                        span: Span::new(0, 6),
                    },
                    JasmToken {
                        kind: JasmTokenKind::Public,
                        span: Span::new(7, 13),
                    },
                    JasmToken {
                        kind: JasmTokenKind::Eof,
                        span: Span::new(13, 13),
                    },
                ]
            );
        }

        #[test]
        fn test_string_followed_by_directive() {
            let mut lexer = JasmLexer::new(r#""test" .class"#);
            let tokens = lexer.tokenize().unwrap();

            assert_eq!(
                tokens,
                vec![
                    JasmToken {
                        kind: JasmTokenKind::StringLiteral("test".to_string()),
                        span: Span::new(0, 6),
                    },
                    JasmToken {
                        kind: JasmTokenKind::DotClass,
                        span: Span::new(7, 13),
                    },
                    JasmToken {
                        kind: JasmTokenKind::Eof,
                        span: Span::new(13, 13),
                    },
                ]
            );
        }

        #[test]
        fn test_empty_string_at_eof() {
            let mut lexer = JasmLexer::new(r#""""#);
            let tokens = lexer.tokenize().unwrap();

            assert_eq!(
                tokens,
                vec![
                    JasmToken {
                        kind: JasmTokenKind::StringLiteral("".to_string()),
                        span: Span::new(0, 2),
                    },
                    JasmToken {
                        kind: JasmTokenKind::Eof,
                        span: Span::new(2, 2),
                    },
                ]
            );
        }

        #[test]
        fn test_strings_separated_by_newline() {
            let mut lexer = JasmLexer::new("\"str1\"\n\"str2\"");
            let tokens = lexer.tokenize().unwrap();

            assert_eq!(
                tokens,
                vec![
                    JasmToken {
                        kind: JasmTokenKind::StringLiteral("str1".to_string()),
                        span: Span::new(0, 6),
                    },
                    JasmToken {
                        kind: JasmTokenKind::Newline,
                        span: Span::new(6, 7),
                    },
                    JasmToken {
                        kind: JasmTokenKind::StringLiteral("str2".to_string()),
                        span: Span::new(7, 13),
                    },
                    JasmToken {
                        kind: JasmTokenKind::Eof,
                        span: Span::new(13, 13),
                    },
                ]
            );
        }
    }

    mod string_comment_interaction {
        use super::*;

        #[test]
        fn test_string_then_comment() {
            // Comment after string should be skipped
            let mut lexer = JasmLexer::new(r#""hello" ; this is a comment"#);
            let tokens = lexer.tokenize().unwrap();

            assert_eq!(
                tokens,
                vec![
                    JasmToken {
                        kind: JasmTokenKind::StringLiteral("hello".to_string()),
                        span: Span::new(0, 7),
                    },
                    JasmToken {
                        kind: JasmTokenKind::Eof,
                        span: Span::new(27, 27),
                    },
                ]
            );
        }

        #[test]
        fn test_string_with_semicolon_inside() {
            // Semicolon inside string is NOT a comment
            let mut lexer = JasmLexer::new(r#""hello ; not a comment""#);
            let tokens = lexer.tokenize().unwrap();

            assert_eq!(
                tokens,
                vec![
                    JasmToken {
                        kind: JasmTokenKind::StringLiteral("hello ; not a comment".to_string()),
                        span: Span::new(0, 23),
                    },
                    JasmToken {
                        kind: JasmTokenKind::Eof,
                        span: Span::new(23, 23),
                    },
                ]
            );
        }

        #[test]
        fn test_string_directly_followed_by_comment() {
            // No space between string and comment
            let mut lexer = JasmLexer::new(r#""hello"; comment"#);
            let tokens = lexer.tokenize().unwrap();

            assert_eq!(
                tokens,
                vec![
                    JasmToken {
                        kind: JasmTokenKind::StringLiteral("hello".to_string()),
                        span: Span::new(0, 7),
                    },
                    JasmToken {
                        kind: JasmTokenKind::Eof,
                        span: Span::new(16, 16),
                    },
                ]
            );
        }

        #[test]
        fn test_string_with_multiple_semicolons_inside() {
            let mut lexer = JasmLexer::new(r#""a;b;c;d""#);
            let tokens = lexer.tokenize().unwrap();

            assert_eq!(
                tokens,
                vec![
                    JasmToken {
                        kind: JasmTokenKind::StringLiteral("a;b;c;d".to_string()),
                        span: Span::new(0, 9),
                    },
                    JasmToken {
                        kind: JasmTokenKind::Eof,
                        span: Span::new(9, 9),
                    },
                ]
            );
        }
    }

    mod errors {
        use super::*;
        use rstest::rstest;

        #[rstest]
        #[case(r#""hello"#, 0, "unterminated string at EOF")]
        #[case("\"hello\nworld\"", 0, "literal newline in string")]
        #[case("\"hello\rworld\"", 0, "literal carriage return in string")]
        #[case(r#""hello\"#, 0, "string ending with backslash at EOF")]
        #[case(r#"""#, 0, "unterminated empty string")]
        #[case(".class \"unterminated", 7, "unterminated string after token")]
        fn test_unterminated_string(
            #[case] input: &str,
            #[case] expected_pos: usize,
            #[case] _description: &str,
        ) {
            let mut lexer = JasmLexer::new(input);
            let result = lexer.tokenize();

            assert_eq!(
                result,
                Err(LexerError::UnterminatedString(Span::new(
                    expected_pos,
                    expected_pos + 1
                )))
            );
        }
    }
}

mod comments {
    use super::*;

    #[test]
    fn test_comment_after_whitespace_is_skipped() {
        let mut lexer = JasmLexer::new(".class ; this is a comment");
        let tokens = lexer.tokenize().unwrap();

        assert_eq!(
            tokens,
            vec![
                JasmToken {
                    kind: JasmTokenKind::DotClass,
                    span: Span::new(0, 6),
                },
                JasmToken {
                    kind: JasmTokenKind::Eof,
                    span: Span::new(26, 26),
                },
            ]
        );
    }

    #[test]
    fn test_comment_with_special_characters() {
        let mut lexer = JasmLexer::new(".class ; !@#$%^&*()_+-=[]{}|:\"'<>?,./~`");
        let tokens = lexer.tokenize().unwrap();

        assert_eq!(
            tokens,
            vec![
                JasmToken {
                    kind: JasmTokenKind::DotClass,
                    span: Span::new(0, 6),
                },
                JasmToken {
                    kind: JasmTokenKind::Eof,
                    span: Span::new(39, 39),
                },
            ]
        );
    }

    #[test]
    fn test_comment_ends_at_newline() {
        let mut lexer = JasmLexer::new(".class ; comment\n.super");
        let tokens = lexer.tokenize().unwrap();

        assert_eq!(
            tokens,
            vec![
                JasmToken {
                    kind: JasmTokenKind::DotClass,
                    span: Span::new(0, 6),
                },
                JasmToken {
                    kind: JasmTokenKind::Newline,
                    span: Span::new(16, 17),
                },
                JasmToken {
                    kind: JasmTokenKind::DotSuper,
                    span: Span::new(17, 23),
                },
                JasmToken {
                    kind: JasmTokenKind::Eof,
                    span: Span::new(23, 23),
                },
            ]
        );
    }

    #[test]
    fn test_empty_comment() {
        let mut lexer = JasmLexer::new(".class ;\n.super");
        let tokens = lexer.tokenize().unwrap();

        assert_eq!(
            tokens,
            vec![
                JasmToken {
                    kind: JasmTokenKind::DotClass,
                    span: Span::new(0, 6),
                },
                JasmToken {
                    kind: JasmTokenKind::Newline,
                    span: Span::new(8, 9),
                },
                JasmToken {
                    kind: JasmTokenKind::DotSuper,
                    span: Span::new(9, 15),
                },
                JasmToken {
                    kind: JasmTokenKind::Eof,
                    span: Span::new(15, 15),
                },
            ]
        );
    }

    #[test]
    fn test_comment_only_line() {
        let mut lexer = JasmLexer::new("; only a comment");
        let tokens = lexer.tokenize().unwrap();

        assert_eq!(
            tokens,
            vec![JasmToken {
                kind: JasmTokenKind::Eof,
                span: Span::new(16, 16),
            },]
        );
    }

    #[test]
    fn test_multiple_comments_on_different_lines() {
        let mut lexer = JasmLexer::new(".class ; comment 1\n; comment 2\n.super");
        let tokens = lexer.tokenize().unwrap();

        assert_eq!(
            tokens,
            vec![
                JasmToken {
                    kind: JasmTokenKind::DotClass,
                    span: Span::new(0, 6),
                },
                JasmToken {
                    kind: JasmTokenKind::Newline,
                    span: Span::new(18, 19),
                },
                JasmToken {
                    kind: JasmTokenKind::Newline,
                    span: Span::new(30, 31),
                },
                JasmToken {
                    kind: JasmTokenKind::DotSuper,
                    span: Span::new(31, 37),
                },
                JasmToken {
                    kind: JasmTokenKind::Eof,
                    span: Span::new(37, 37),
                },
            ]
        );
    }

    #[test]
    fn test_comment_at_end_of_file() {
        let mut lexer = JasmLexer::new(".class ; comment at eof");
        let tokens = lexer.tokenize().unwrap();

        assert_eq!(
            tokens,
            vec![
                JasmToken {
                    kind: JasmTokenKind::DotClass,
                    span: Span::new(0, 6),
                },
                JasmToken {
                    kind: JasmTokenKind::Eof,
                    span: Span::new(23, 23),
                },
            ]
        );
    }

    #[test]
    fn test_identifier_with_semicolon_not_a_comment() {
        // Semicolon is part of identifier (no whitespace before semicolon)
        let mut lexer = JasmLexer::new("identifier;notcomment");
        let tokens = lexer.tokenize().unwrap();

        assert_eq!(
            tokens,
            vec![
                JasmToken {
                    kind: JasmTokenKind::Identifier("identifier;notcomment".to_string()),
                    span: Span::new(0, 21),
                },
                JasmToken {
                    kind: JasmTokenKind::Eof,
                    span: Span::new(21, 21),
                },
            ]
        );
    }
}

mod whitespace {
    use super::*;

    mod basic_whitespace {
        use super::*;

        #[test]
        fn test_spaces_between_tokens() {
            let mut lexer = JasmLexer::new(".class    public");
            let tokens = lexer.tokenize().unwrap();

            assert_eq!(
                tokens,
                vec![
                    JasmToken {
                        kind: JasmTokenKind::DotClass,
                        span: Span::new(0, 6),
                    },
                    JasmToken {
                        kind: JasmTokenKind::Public,
                        span: Span::new(10, 16),
                    },
                    JasmToken {
                        kind: JasmTokenKind::Eof,
                        span: Span::new(16, 16),
                    },
                ]
            );
        }

        #[test]
        fn test_tabs_between_tokens() {
            let mut lexer = JasmLexer::new(".class\t\tpublic");
            let tokens = lexer.tokenize().unwrap();

            assert_eq!(
                tokens,
                vec![
                    JasmToken {
                        kind: JasmTokenKind::DotClass,
                        span: Span::new(0, 6),
                    },
                    JasmToken {
                        kind: JasmTokenKind::Public,
                        span: Span::new(8, 14),
                    },
                    JasmToken {
                        kind: JasmTokenKind::Eof,
                        span: Span::new(14, 14),
                    },
                ]
            );
        }

        #[test]
        fn test_mixed_spaces_and_tabs() {
            let mut lexer = JasmLexer::new(".class \t \t public");
            let tokens = lexer.tokenize().unwrap();

            assert_eq!(
                tokens,
                vec![
                    JasmToken {
                        kind: JasmTokenKind::DotClass,
                        span: Span::new(0, 6),
                    },
                    JasmToken {
                        kind: JasmTokenKind::Public,
                        span: Span::new(11, 17),
                    },
                    JasmToken {
                        kind: JasmTokenKind::Eof,
                        span: Span::new(17, 17),
                    },
                ]
            );
        }

        #[test]
        fn test_carriage_return_skipped() {
            let mut lexer = JasmLexer::new(".class\r\rpublic");
            let tokens = lexer.tokenize().unwrap();

            assert_eq!(
                tokens,
                vec![
                    JasmToken {
                        kind: JasmTokenKind::DotClass,
                        span: Span::new(0, 6),
                    },
                    JasmToken {
                        kind: JasmTokenKind::Public,
                        span: Span::new(8, 14),
                    },
                    JasmToken {
                        kind: JasmTokenKind::Eof,
                        span: Span::new(14, 14),
                    },
                ]
            );
        }

        #[test]
        fn test_leading_whitespace() {
            let mut lexer = JasmLexer::new("   \t  .class");
            let tokens = lexer.tokenize().unwrap();

            assert_eq!(
                tokens,
                vec![
                    JasmToken {
                        kind: JasmTokenKind::DotClass,
                        span: Span::new(6, 12),
                    },
                    JasmToken {
                        kind: JasmTokenKind::Eof,
                        span: Span::new(12, 12),
                    },
                ]
            );
        }

        #[test]
        fn test_trailing_whitespace() {
            let mut lexer = JasmLexer::new(".class   \t  ");
            let tokens = lexer.tokenize().unwrap();

            assert_eq!(
                tokens,
                vec![
                    JasmToken {
                        kind: JasmTokenKind::DotClass,
                        span: Span::new(0, 6),
                    },
                    JasmToken {
                        kind: JasmTokenKind::Eof,
                        span: Span::new(12, 12),
                    },
                ]
            );
        }
    }

    mod newlines {
        use super::*;

        #[test]
        fn test_single_newline() {
            let mut lexer = JasmLexer::new(".class\n.super");
            let tokens = lexer.tokenize().unwrap();

            assert_eq!(
                tokens,
                vec![
                    JasmToken {
                        kind: JasmTokenKind::DotClass,
                        span: Span::new(0, 6),
                    },
                    JasmToken {
                        kind: JasmTokenKind::Newline,
                        span: Span::new(6, 7),
                    },
                    JasmToken {
                        kind: JasmTokenKind::DotSuper,
                        span: Span::new(7, 13),
                    },
                    JasmToken {
                        kind: JasmTokenKind::Eof,
                        span: Span::new(13, 13),
                    },
                ]
            );
        }

        #[test]
        fn test_multiple_consecutive_newlines() {
            let mut lexer = JasmLexer::new(".class\n\n\n.super");
            let tokens = lexer.tokenize().unwrap();

            assert_eq!(
                tokens,
                vec![
                    JasmToken {
                        kind: JasmTokenKind::DotClass,
                        span: Span::new(0, 6),
                    },
                    JasmToken {
                        kind: JasmTokenKind::Newline,
                        span: Span::new(6, 7),
                    },
                    JasmToken {
                        kind: JasmTokenKind::Newline,
                        span: Span::new(7, 8),
                    },
                    JasmToken {
                        kind: JasmTokenKind::Newline,
                        span: Span::new(8, 9),
                    },
                    JasmToken {
                        kind: JasmTokenKind::DotSuper,
                        span: Span::new(9, 15),
                    },
                    JasmToken {
                        kind: JasmTokenKind::Eof,
                        span: Span::new(15, 15),
                    },
                ]
            );
        }

        #[test]
        fn test_newline_with_surrounding_whitespace() {
            // Whitespace on same line is skipped, newline is tokenized
            let mut lexer = JasmLexer::new(".class   \n   .super");
            let tokens = lexer.tokenize().unwrap();

            assert_eq!(
                tokens,
                vec![
                    JasmToken {
                        kind: JasmTokenKind::DotClass,
                        span: Span::new(0, 6),
                    },
                    JasmToken {
                        kind: JasmTokenKind::Newline,
                        span: Span::new(9, 10),
                    },
                    JasmToken {
                        kind: JasmTokenKind::DotSuper,
                        span: Span::new(13, 19),
                    },
                    JasmToken {
                        kind: JasmTokenKind::Eof,
                        span: Span::new(19, 19),
                    },
                ]
            );
        }

        #[test]
        fn test_crlf_line_ending() {
            // Windows-style line endings: \r is skipped, \n is tokenized
            let mut lexer = JasmLexer::new(".class\r\n.super");
            let tokens = lexer.tokenize().unwrap();

            assert_eq!(
                tokens,
                vec![
                    JasmToken {
                        kind: JasmTokenKind::DotClass,
                        span: Span::new(0, 6),
                    },
                    JasmToken {
                        kind: JasmTokenKind::Newline,
                        span: Span::new(7, 8),
                    },
                    JasmToken {
                        kind: JasmTokenKind::DotSuper,
                        span: Span::new(8, 14),
                    },
                    JasmToken {
                        kind: JasmTokenKind::Eof,
                        span: Span::new(14, 14),
                    },
                ]
            );
        }

        #[test]
        fn test_trailing_newline_at_eof() {
            let mut lexer = JasmLexer::new(".class\n");
            let tokens = lexer.tokenize().unwrap();

            assert_eq!(
                tokens,
                vec![
                    JasmToken {
                        kind: JasmTokenKind::DotClass,
                        span: Span::new(0, 6),
                    },
                    JasmToken {
                        kind: JasmTokenKind::Newline,
                        span: Span::new(6, 7),
                    },
                    JasmToken {
                        kind: JasmTokenKind::Eof,
                        span: Span::new(7, 7),
                    },
                ]
            );
        }

        #[test]
        fn test_leading_newline() {
            let mut lexer = JasmLexer::new("\n.class");
            let tokens = lexer.tokenize().unwrap();

            assert_eq!(
                tokens,
                vec![
                    JasmToken {
                        kind: JasmTokenKind::Newline,
                        span: Span::new(0, 1),
                    },
                    JasmToken {
                        kind: JasmTokenKind::DotClass,
                        span: Span::new(1, 7),
                    },
                    JasmToken {
                        kind: JasmTokenKind::Eof,
                        span: Span::new(7, 7),
                    },
                ]
            );
        }
    }

    mod empty_input {
        use super::*;

        #[test]
        fn test_empty_input() {
            let mut lexer = JasmLexer::new("");
            let tokens = lexer.tokenize().unwrap();

            assert_eq!(
                tokens,
                vec![JasmToken {
                    kind: JasmTokenKind::Eof,
                    span: Span::new(0, 0),
                },]
            );
        }

        #[test]
        fn test_whitespace_only_input() {
            let mut lexer = JasmLexer::new("   \t\t   ");
            let tokens = lexer.tokenize().unwrap();

            assert_eq!(
                tokens,
                vec![JasmToken {
                    kind: JasmTokenKind::Eof,
                    span: Span::new(8, 8),
                },]
            );
        }

        #[test]
        fn test_newlines_only_input() {
            let mut lexer = JasmLexer::new("\n\n\n");
            let tokens = lexer.tokenize().unwrap();

            assert_eq!(
                tokens,
                vec![
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
                        kind: JasmTokenKind::Eof,
                        span: Span::new(3, 3),
                    },
                ]
            );
        }
    }
}

mod complex_sequences {
    use super::*;

    #[test]
    fn test_all_token_types_mixed() {
        // directive + keyword + keyword + identifier + string + newline
        let mut lexer = JasmLexer::new(".method public static main \"hello\"\n.end");
        let tokens = lexer.tokenize().unwrap();

        assert_eq!(
            tokens,
            vec![
                JasmToken {
                    kind: JasmTokenKind::DotMethod,
                    span: Span::new(0, 7),
                },
                JasmToken {
                    kind: JasmTokenKind::Public,
                    span: Span::new(8, 14),
                },
                JasmToken {
                    kind: JasmTokenKind::Static,
                    span: Span::new(15, 21),
                },
                JasmToken {
                    kind: JasmTokenKind::Identifier("main".to_string()),
                    span: Span::new(22, 26),
                },
                JasmToken {
                    kind: JasmTokenKind::StringLiteral("hello".to_string()),
                    span: Span::new(27, 34),
                },
                JasmToken {
                    kind: JasmTokenKind::Newline,
                    span: Span::new(34, 35),
                },
                JasmToken {
                    kind: JasmTokenKind::DotEnd,
                    span: Span::new(35, 39),
                },
                JasmToken {
                    kind: JasmTokenKind::Eof,
                    span: Span::new(39, 39),
                },
            ]
        );
    }

    #[test]
    fn test_string_directive_keyword_identifier() {
        let mut lexer = JasmLexer::new("\"test\" .class public myIdentifier");
        let tokens = lexer.tokenize().unwrap();

        assert_eq!(
            tokens,
            vec![
                JasmToken {
                    kind: JasmTokenKind::StringLiteral("test".to_string()),
                    span: Span::new(0, 6),
                },
                JasmToken {
                    kind: JasmTokenKind::DotClass,
                    span: Span::new(7, 13),
                },
                JasmToken {
                    kind: JasmTokenKind::Public,
                    span: Span::new(14, 20),
                },
                JasmToken {
                    kind: JasmTokenKind::Identifier("myIdentifier".to_string()),
                    span: Span::new(21, 33),
                },
                JasmToken {
                    kind: JasmTokenKind::Eof,
                    span: Span::new(33, 33),
                },
            ]
        );
    }

    #[test]
    fn test_comments_between_tokens() {
        let mut lexer = JasmLexer::new(".class ; first comment\n; second comment\n.super");
        let tokens = lexer.tokenize().unwrap();

        assert_eq!(
            tokens,
            vec![
                JasmToken {
                    kind: JasmTokenKind::DotClass,
                    span: Span::new(0, 6),
                },
                JasmToken {
                    kind: JasmTokenKind::Newline,
                    span: Span::new(22, 23),
                },
                JasmToken {
                    kind: JasmTokenKind::Newline,
                    span: Span::new(39, 40),
                },
                JasmToken {
                    kind: JasmTokenKind::DotSuper,
                    span: Span::new(40, 46),
                },
                JasmToken {
                    kind: JasmTokenKind::Eof,
                    span: Span::new(46, 46),
                },
            ]
        );
    }

    #[test]
    fn test_keyword_as_part_of_identifier() {
        // "publicStatic" should be single identifier, not Public + Static
        let mut lexer = JasmLexer::new("publicStatic staticPublic");
        let tokens = lexer.tokenize().unwrap();

        assert_eq!(
            tokens,
            vec![
                JasmToken {
                    kind: JasmTokenKind::Identifier("publicStatic".to_string()),
                    span: Span::new(0, 12),
                },
                JasmToken {
                    kind: JasmTokenKind::Identifier("staticPublic".to_string()),
                    span: Span::new(13, 25),
                },
                JasmToken {
                    kind: JasmTokenKind::Eof,
                    span: Span::new(25, 25),
                },
            ]
        );
    }

    #[test]
    fn test_keyword_case_sensitivity() {
        // Keywords are case-sensitive: Public, PUBLIC, STATIC should be identifiers
        let mut lexer = JasmLexer::new("Public PUBLIC Static STATIC public static");
        let tokens = lexer.tokenize().unwrap();

        assert_eq!(
            tokens,
            vec![
                JasmToken {
                    kind: JasmTokenKind::Identifier("Public".to_string()),
                    span: Span::new(0, 6),
                },
                JasmToken {
                    kind: JasmTokenKind::Identifier("PUBLIC".to_string()),
                    span: Span::new(7, 13),
                },
                JasmToken {
                    kind: JasmTokenKind::Identifier("Static".to_string()),
                    span: Span::new(14, 20),
                },
                JasmToken {
                    kind: JasmTokenKind::Identifier("STATIC".to_string()),
                    span: Span::new(21, 27),
                },
                JasmToken {
                    kind: JasmTokenKind::Public,
                    span: Span::new(28, 34),
                },
                JasmToken {
                    kind: JasmTokenKind::Static,
                    span: Span::new(35, 41),
                },
                JasmToken {
                    kind: JasmTokenKind::Eof,
                    span: Span::new(41, 41),
                },
            ]
        );
    }

    #[test]
    fn test_realistic_jasmin_code() {
        let input = r#".class public HelloWorld
.super java/lang/Object

.method public static main([Ljava/lang/String;)V
.code
.end method"#;

        let mut lexer = JasmLexer::new(input);
        let tokens = lexer.tokenize().unwrap();

        // Check first few tokens
        assert_eq!(tokens[0].kind, JasmTokenKind::DotClass);
        assert_eq!(tokens[1].kind, JasmTokenKind::Public);
        assert_eq!(
            tokens[2].kind,
            JasmTokenKind::Identifier("HelloWorld".to_string())
        );
        assert_eq!(tokens[3].kind, JasmTokenKind::Newline);
        assert_eq!(tokens[4].kind, JasmTokenKind::DotSuper);
        assert_eq!(
            tokens[5].kind,
            JasmTokenKind::Identifier("java/lang/Object".to_string())
        );

        // Check last token is Eof
        assert_eq!(tokens.last().unwrap().kind, JasmTokenKind::Eof);
    }

    #[test]
    fn test_realistic_jasmin_code_with_integers() {
        // Full example with integers
        let input = r#".class public HelloWorld
.super java/lang/Object

.method public static main([Ljava/lang/String;)V
.code
    iconst_2
    istore_1
.end method"#;

        let mut lexer = JasmLexer::new(input);
        let tokens = lexer.tokenize().unwrap();

        // Verify key tokens
        assert_eq!(tokens[0].kind, JasmTokenKind::DotClass);
        assert_eq!(tokens[1].kind, JasmTokenKind::Public);
        assert_eq!(
            tokens[2].kind,
            JasmTokenKind::Identifier("HelloWorld".to_string())
        );

        // Find .code
        let code_idx = tokens
            .iter()
            .position(|t| t.kind == JasmTokenKind::DotCode)
            .unwrap();
        assert!(code_idx > 0);

        assert_eq!(tokens.last().unwrap().kind, JasmTokenKind::Eof);
    }

    #[test]
    fn test_method_with_init() {
        let input = r#".method public <init>()V
.code
return
.end method"#;

        let mut lexer = JasmLexer::new(input);
        let tokens = lexer.tokenize().unwrap();

        assert_eq!(tokens[0].kind, JasmTokenKind::DotMethod);
        assert_eq!(tokens[1].kind, JasmTokenKind::Public);
        assert_eq!(
            tokens[2].kind,
            JasmTokenKind::Identifier("<init>".to_string())
        );
        assert_eq!(tokens[3].kind, JasmTokenKind::OpenParen);
        assert_eq!(tokens[4].kind, JasmTokenKind::CloseParen);
        assert_eq!(tokens[5].kind, JasmTokenKind::Identifier("V".to_string()));
    }

    #[test]
    fn test_method_with_clinit() {
        let input = ".method static <clinit>()V\n.end method";

        let mut lexer = JasmLexer::new(input);
        let tokens = lexer.tokenize().unwrap();

        assert_eq!(tokens[0].kind, JasmTokenKind::DotMethod);
        assert_eq!(tokens[1].kind, JasmTokenKind::Static);
        assert_eq!(
            tokens[2].kind,
            JasmTokenKind::Identifier("<clinit>".to_string())
        );
        assert_eq!(tokens[3].kind, JasmTokenKind::OpenParen);
        assert_eq!(tokens[4].kind, JasmTokenKind::CloseParen);
        assert_eq!(tokens[5].kind, JasmTokenKind::Identifier("V".to_string()));
    }

    #[test]
    fn test_complex_method_with_all_features() {
        // Method with parens, brackets, integers, descriptors
        let input = r#".method public static process([II)I
.code
.end method"#;

        let mut lexer = JasmLexer::new(input);
        let tokens = lexer.tokenize().unwrap();

        // .method public static process
        assert_eq!(tokens[0].kind, JasmTokenKind::DotMethod);
        assert_eq!(tokens[1].kind, JasmTokenKind::Public);
        assert_eq!(tokens[2].kind, JasmTokenKind::Static);
        assert_eq!(
            tokens[3].kind,
            JasmTokenKind::Identifier("process".to_string())
        );

        // ([II)I - parameter descriptor
        assert_eq!(tokens[4].kind, JasmTokenKind::OpenParen);
        assert_eq!(tokens[5].kind, JasmTokenKind::OpenBracket);
        assert_eq!(tokens[6].kind, JasmTokenKind::Identifier("II".to_string()));
        assert_eq!(tokens[7].kind, JasmTokenKind::CloseParen);
        assert_eq!(tokens[8].kind, JasmTokenKind::Identifier("I".to_string()));

        // .code directive should be present
        assert!(tokens.iter().any(|t| t.kind == JasmTokenKind::DotCode));
    }

    #[test]
    fn test_all_new_features_together() {
        // Combines integers, parens, brackets, init/clinit, .code
        let input = "42 (arg)[type <init> .code -10";

        let mut lexer = JasmLexer::new(input);
        let tokens = lexer.tokenize().unwrap();

        assert_eq!(tokens[0].kind, JasmTokenKind::Integer(42));
        assert_eq!(tokens[1].kind, JasmTokenKind::OpenParen);
        assert_eq!(tokens[2].kind, JasmTokenKind::Identifier("arg".to_string()));
        assert_eq!(tokens[3].kind, JasmTokenKind::CloseParen);
        assert_eq!(tokens[4].kind, JasmTokenKind::OpenBracket);
        assert_eq!(
            tokens[5].kind,
            JasmTokenKind::Identifier("type".to_string())
        );
        assert_eq!(
            tokens[6].kind,
            JasmTokenKind::Identifier("<init>".to_string())
        );
        assert_eq!(tokens[7].kind, JasmTokenKind::DotCode);
        assert_eq!(tokens[8].kind, JasmTokenKind::Integer(-10));
        assert_eq!(tokens[9].kind, JasmTokenKind::Eof);
    }

    #[test]
    fn test_multiple_integers_with_parens() {
        let input = "(1 2 -3)";

        let mut lexer = JasmLexer::new(input);
        let tokens = lexer.tokenize().unwrap();

        assert_eq!(tokens[0].kind, JasmTokenKind::OpenParen);
        assert_eq!(tokens[1].kind, JasmTokenKind::Integer(1));
        assert_eq!(tokens[2].kind, JasmTokenKind::Integer(2));
        assert_eq!(tokens[3].kind, JasmTokenKind::Integer(-3));
        assert_eq!(tokens[4].kind, JasmTokenKind::CloseParen);
        assert_eq!(tokens[5].kind, JasmTokenKind::Eof);
    }
}

mod error_handling {
    use super::*;
    use rstest::rstest;

    #[rstest]
    #[case("@", 0, '@')]
    #[case("#", 0, '#')]
    #[case("$", 0, '$')]
    #[case("%", 0, '%')]
    #[case("&", 0, '&')]
    #[case("]", 0, ']')]
    #[case("{", 0, '{')]
    #[case("}", 0, '}')]
    #[case("=", 0, '=')]
    #[case("+", 0, '+')]
    #[case("*", 0, '*')]
    #[case("!", 0, '!')]
    #[case("~", 0, '~')]
    #[case("`", 0, '`')]
    #[case("|", 0, '|')]
    #[case("^", 0, '^')]
    #[case("<", 0, '<')]
    #[case(">", 0, '>')]
    #[case("?", 0, '?')]
    #[case(",", 0, ',')]
    #[case(":", 0, ':')]
    #[case("/", 0, '/')]
    #[case("'", 0, '\'')]
    fn test_unexpected_character(#[case] input: &str, #[case] pos: usize, #[case] ch: char) {
        let mut lexer = JasmLexer::new(input);
        let result = lexer.tokenize();

        assert!(matches!(
            result,
            Err(LexerError::UnexpectedChar(ref span, c, _)) if span.start == pos && c == ch
        ));
    }

    #[test]
    fn test_unexpected_char_after_valid_token() {
        let mut lexer = JasmLexer::new(".class @");
        let result = lexer.tokenize();

        assert!(
            matches!(result, Err(LexerError::UnexpectedChar(ref span, '@', _)) if span.start == 7)
        );
    }

    #[test]
    fn test_unexpected_char_between_valid_tokens() {
        let mut lexer = JasmLexer::new(".class @ .super");
        let result = lexer.tokenize();

        assert!(
            matches!(result, Err(LexerError::UnexpectedChar(ref span, '@', _)) if span.start == 7)
        );
    }

    #[test]
    fn test_unexpected_char_on_second_line() {
        let mut lexer = JasmLexer::new(".class\n@");
        let result = lexer.tokenize();

        assert!(
            matches!(result, Err(LexerError::UnexpectedChar(ref span, '@', _)) if span.start == 7)
        );
    }

    #[test]
    fn test_unexpected_char_error_message_context() {
        let mut lexer = JasmLexer::new("@");
        let result = lexer.tokenize();

        if let Err(ref err @ LexerError::UnexpectedChar(_, _, _)) = result {
            assert!(
                err.note()
                    .expect("Expected note for UnexpectedChar")
                    .contains("Unexpected character")
            );
        } else {
            panic!("Expected UnexpectedChar error");
        }
    }
}

mod code_directive {
    use super::*;

    #[test]
    fn test_code_directive_basic() {
        let mut lexer = JasmLexer::new(".code");
        let tokens = lexer.tokenize().unwrap();

        assert_eq!(
            tokens,
            vec![
                JasmToken {
                    kind: JasmTokenKind::DotCode,
                    span: Span::new(0, 5),
                },
                JasmToken {
                    kind: JasmTokenKind::Eof,
                    span: Span::new(5, 5),
                },
            ]
        );
    }

    #[test]
    fn test_code_directive_in_sequence() {
        let mut lexer = JasmLexer::new(".method\n.code\n.end");
        let tokens = lexer.tokenize().unwrap();

        assert_eq!(
            tokens,
            vec![
                JasmToken {
                    kind: JasmTokenKind::DotMethod,
                    span: Span::new(0, 7),
                },
                JasmToken {
                    kind: JasmTokenKind::Newline,
                    span: Span::new(7, 8),
                },
                JasmToken {
                    kind: JasmTokenKind::DotCode,
                    span: Span::new(8, 13),
                },
                JasmToken {
                    kind: JasmTokenKind::Newline,
                    span: Span::new(13, 14),
                },
                JasmToken {
                    kind: JasmTokenKind::DotEnd,
                    span: Span::new(14, 18),
                },
                JasmToken {
                    kind: JasmTokenKind::Eof,
                    span: Span::new(18, 18),
                },
            ]
        );
    }

    #[test]
    fn test_code_directive_with_identifier() {
        let mut lexer = JasmLexer::new(".code stack");
        let tokens = lexer.tokenize().unwrap();

        assert_eq!(
            tokens,
            vec![
                JasmToken {
                    kind: JasmTokenKind::DotCode,
                    span: Span::new(0, 5),
                },
                JasmToken {
                    kind: JasmTokenKind::Identifier("stack".to_string()),
                    span: Span::new(6, 11),
                },
                JasmToken {
                    kind: JasmTokenKind::Eof,
                    span: Span::new(11, 11),
                },
            ]
        );
    }

    #[test]
    fn test_all_directives_together() {
        let mut lexer = JasmLexer::new(".class .super .method .code .end");
        let tokens = lexer.tokenize().unwrap();

        assert_eq!(
            tokens,
            vec![
                JasmToken {
                    kind: JasmTokenKind::DotClass,
                    span: Span::new(0, 6),
                },
                JasmToken {
                    kind: JasmTokenKind::DotSuper,
                    span: Span::new(7, 13),
                },
                JasmToken {
                    kind: JasmTokenKind::DotMethod,
                    span: Span::new(14, 21),
                },
                JasmToken {
                    kind: JasmTokenKind::DotCode,
                    span: Span::new(22, 27),
                },
                JasmToken {
                    kind: JasmTokenKind::DotEnd,
                    span: Span::new(28, 32),
                },
                JasmToken {
                    kind: JasmTokenKind::Eof,
                    span: Span::new(32, 32),
                },
            ]
        );
    }
}

mod brackets_and_parens {
    use super::*;

    #[test]
    fn test_open_paren_alone() {
        let mut lexer = JasmLexer::new("(");
        let tokens = lexer.tokenize().unwrap();

        assert_eq!(
            tokens,
            vec![
                JasmToken {
                    kind: JasmTokenKind::OpenParen,
                    span: Span::new(0, 1),
                },
                JasmToken {
                    kind: JasmTokenKind::Eof,
                    span: Span::new(1, 1),
                },
            ]
        );
    }

    #[test]
    fn test_close_paren_alone() {
        let mut lexer = JasmLexer::new(")");
        let tokens = lexer.tokenize().unwrap();

        assert_eq!(
            tokens,
            vec![
                JasmToken {
                    kind: JasmTokenKind::CloseParen,
                    span: Span::new(0, 1),
                },
                JasmToken {
                    kind: JasmTokenKind::Eof,
                    span: Span::new(1, 1),
                },
            ]
        );
    }

    #[test]
    fn test_open_bracket_alone() {
        let mut lexer = JasmLexer::new("[");
        let tokens = lexer.tokenize().unwrap();

        assert_eq!(
            tokens,
            vec![
                JasmToken {
                    kind: JasmTokenKind::OpenBracket,
                    span: Span::new(0, 1),
                },
                JasmToken {
                    kind: JasmTokenKind::Eof,
                    span: Span::new(1, 1),
                },
            ]
        );
    }

    #[test]
    fn test_parens_in_sequence() {
        let mut lexer = JasmLexer::new("()");
        let tokens = lexer.tokenize().unwrap();

        assert_eq!(
            tokens,
            vec![
                JasmToken {
                    kind: JasmTokenKind::OpenParen,
                    span: Span::new(0, 1),
                },
                JasmToken {
                    kind: JasmTokenKind::CloseParen,
                    span: Span::new(1, 2),
                },
                JasmToken {
                    kind: JasmTokenKind::Eof,
                    span: Span::new(2, 2),
                },
            ]
        );
    }

    #[test]
    fn test_parens_with_content() {
        let mut lexer = JasmLexer::new("(I)V");
        let tokens = lexer.tokenize().unwrap();

        assert_eq!(
            tokens,
            vec![
                JasmToken {
                    kind: JasmTokenKind::OpenParen,
                    span: Span::new(0, 1),
                },
                JasmToken {
                    kind: JasmTokenKind::Identifier("I".to_string()),
                    span: Span::new(1, 2),
                },
                JasmToken {
                    kind: JasmTokenKind::CloseParen,
                    span: Span::new(2, 3),
                },
                JasmToken {
                    kind: JasmTokenKind::Identifier("V".to_string()),
                    span: Span::new(3, 4),
                },
                JasmToken {
                    kind: JasmTokenKind::Eof,
                    span: Span::new(4, 4),
                },
            ]
        );
    }

    #[test]
    fn test_bracket_with_type() {
        let mut lexer = JasmLexer::new("[I");
        let tokens = lexer.tokenize().unwrap();

        assert_eq!(
            tokens,
            vec![
                JasmToken {
                    kind: JasmTokenKind::OpenBracket,
                    span: Span::new(0, 1),
                },
                JasmToken {
                    kind: JasmTokenKind::Identifier("I".to_string()),
                    span: Span::new(1, 2),
                },
                JasmToken {
                    kind: JasmTokenKind::Eof,
                    span: Span::new(2, 2),
                },
            ]
        );
    }

    #[test]
    fn test_complex_method_signature() {
        // ([Ljava/lang/String;)V
        let mut lexer = JasmLexer::new("([Ljava/lang/String;)V");
        let tokens = lexer.tokenize().unwrap();

        assert_eq!(
            tokens,
            vec![
                JasmToken {
                    kind: JasmTokenKind::OpenParen,
                    span: Span::new(0, 1),
                },
                JasmToken {
                    kind: JasmTokenKind::OpenBracket,
                    span: Span::new(1, 2),
                },
                JasmToken {
                    kind: JasmTokenKind::Identifier("Ljava/lang/String;".to_string()),
                    span: Span::new(2, 20),
                },
                JasmToken {
                    kind: JasmTokenKind::CloseParen,
                    span: Span::new(20, 21),
                },
                JasmToken {
                    kind: JasmTokenKind::Identifier("V".to_string()),
                    span: Span::new(21, 22),
                },
                JasmToken {
                    kind: JasmTokenKind::Eof,
                    span: Span::new(22, 22),
                },
            ]
        );
    }

    #[test]
    fn test_parens_with_whitespace() {
        let mut lexer = JasmLexer::new("( I ) V");
        let tokens = lexer.tokenize().unwrap();

        assert_eq!(
            tokens,
            vec![
                JasmToken {
                    kind: JasmTokenKind::OpenParen,
                    span: Span::new(0, 1),
                },
                JasmToken {
                    kind: JasmTokenKind::Identifier("I".to_string()),
                    span: Span::new(2, 3),
                },
                JasmToken {
                    kind: JasmTokenKind::CloseParen,
                    span: Span::new(4, 5),
                },
                JasmToken {
                    kind: JasmTokenKind::Identifier("V".to_string()),
                    span: Span::new(6, 7),
                },
                JasmToken {
                    kind: JasmTokenKind::Eof,
                    span: Span::new(7, 7),
                },
            ]
        );
    }

    #[test]
    fn test_string_followed_by_parens() {
        let mut lexer = JasmLexer::new("\"hello\"()");
        let tokens = lexer.tokenize().unwrap();

        assert_eq!(
            tokens,
            vec![
                JasmToken {
                    kind: JasmTokenKind::StringLiteral("hello".to_string()),
                    span: Span::new(0, 7),
                },
                JasmToken {
                    kind: JasmTokenKind::OpenParen,
                    span: Span::new(7, 8),
                },
                JasmToken {
                    kind: JasmTokenKind::CloseParen,
                    span: Span::new(8, 9),
                },
                JasmToken {
                    kind: JasmTokenKind::Eof,
                    span: Span::new(9, 9),
                },
            ]
        );
    }

    #[test]
    fn test_identifier_then_parens_no_space() {
        let mut lexer = JasmLexer::new("main()");
        let tokens = lexer.tokenize().unwrap();

        assert_eq!(
            tokens,
            vec![
                JasmToken {
                    kind: JasmTokenKind::Identifier("main".to_string()),
                    span: Span::new(0, 4),
                },
                JasmToken {
                    kind: JasmTokenKind::OpenParen,
                    span: Span::new(4, 5),
                },
                JasmToken {
                    kind: JasmTokenKind::CloseParen,
                    span: Span::new(5, 6),
                },
                JasmToken {
                    kind: JasmTokenKind::Eof,
                    span: Span::new(6, 6),
                },
            ]
        );
    }

    #[test]
    fn test_multiple_brackets() {
        let mut lexer = JasmLexer::new("[[I");
        let tokens = lexer.tokenize().unwrap();

        assert_eq!(
            tokens,
            vec![
                JasmToken {
                    kind: JasmTokenKind::OpenBracket,
                    span: Span::new(0, 1),
                },
                JasmToken {
                    kind: JasmTokenKind::OpenBracket,
                    span: Span::new(1, 2),
                },
                JasmToken {
                    kind: JasmTokenKind::Identifier("I".to_string()),
                    span: Span::new(2, 3),
                },
                JasmToken {
                    kind: JasmTokenKind::Eof,
                    span: Span::new(3, 3),
                },
            ]
        );
    }
}

mod integers {
    use super::*;
    use rstest::rstest;

    mod success {
        use super::*;

        #[rstest]
        #[case("0", 0)]
        #[case("1", 1)]
        #[case("42", 42)]
        #[case("123", 123)]
        #[case("-1", -1)]
        #[case("-42", -42)]
        #[case("-0", 0)]
        #[case("007", 7)]
        #[case("0000", 0)]
        #[case("2147483647", i32::MAX)]
        #[case("-2147483648", i32::MIN)]
        fn test_valid_integer(#[case] input: &str, #[case] expected: i32) {
            let mut lexer = JasmLexer::new(input);
            let tokens = lexer.tokenize().unwrap();

            assert_eq!(tokens.len(), 2); // Integer + Eof
            assert_eq!(tokens[0].kind, JasmTokenKind::Integer(expected));
        }

        #[test]
        fn test_integer_with_directive() {
            let mut lexer = JasmLexer::new(".code 5");
            let tokens = lexer.tokenize().unwrap();

            assert_eq!(
                tokens,
                vec![
                    JasmToken {
                        kind: JasmTokenKind::DotCode,
                        span: Span::new(0, 5),
                    },
                    JasmToken {
                        kind: JasmTokenKind::Integer(5),
                        span: Span::new(6, 7),
                    },
                    JasmToken {
                        kind: JasmTokenKind::Eof,
                        span: Span::new(7, 7),
                    },
                ]
            );
        }

        #[test]
        fn test_multiple_integers() {
            let mut lexer = JasmLexer::new("1 2 3");
            let tokens = lexer.tokenize().unwrap();

            assert_eq!(
                tokens,
                vec![
                    JasmToken {
                        kind: JasmTokenKind::Integer(1),
                        span: Span::new(0, 1),
                    },
                    JasmToken {
                        kind: JasmTokenKind::Integer(2),
                        span: Span::new(2, 3),
                    },
                    JasmToken {
                        kind: JasmTokenKind::Integer(3),
                        span: Span::new(4, 5),
                    },
                    JasmToken {
                        kind: JasmTokenKind::Eof,
                        span: Span::new(5, 5),
                    },
                ]
            );
        }

        #[test]
        fn test_integer_followed_by_paren() {
            let mut lexer = JasmLexer::new("5)");
            let tokens = lexer.tokenize().unwrap();

            assert_eq!(
                tokens,
                vec![
                    JasmToken {
                        kind: JasmTokenKind::Integer(5),
                        span: Span::new(0, 1),
                    },
                    JasmToken {
                        kind: JasmTokenKind::CloseParen,
                        span: Span::new(1, 2),
                    },
                    JasmToken {
                        kind: JasmTokenKind::Eof,
                        span: Span::new(2, 2),
                    },
                ]
            );
        }

        #[test]
        fn test_integer_in_parens() {
            let mut lexer = JasmLexer::new("(42)");
            let tokens = lexer.tokenize().unwrap();

            assert_eq!(
                tokens,
                vec![
                    JasmToken {
                        kind: JasmTokenKind::OpenParen,
                        span: Span::new(0, 1),
                    },
                    JasmToken {
                        kind: JasmTokenKind::Integer(42),
                        span: Span::new(1, 3),
                    },
                    JasmToken {
                        kind: JasmTokenKind::CloseParen,
                        span: Span::new(3, 4),
                    },
                    JasmToken {
                        kind: JasmTokenKind::Eof,
                        span: Span::new(4, 4),
                    },
                ]
            );
        }

        #[test]
        fn test_negative_integer_followed_by_bracket() {
            let mut lexer = JasmLexer::new("-5[");
            let tokens = lexer.tokenize().unwrap();

            assert_eq!(
                tokens,
                vec![
                    JasmToken {
                        kind: JasmTokenKind::Integer(-5),
                        span: Span::new(0, 2),
                    },
                    JasmToken {
                        kind: JasmTokenKind::OpenBracket,
                        span: Span::new(2, 3),
                    },
                    JasmToken {
                        kind: JasmTokenKind::Eof,
                        span: Span::new(3, 3),
                    },
                ]
            );
        }
    }

    mod errors {
        use super::*;

        #[rstest]
        #[case("2147483648", 0, 10, "overflow")]
        #[case("-2147483649", 0, 11, "underflow")]
        #[case("123abc", 0, 6, "invalid chars")]
        #[case("--5", 0, 3, "double negative")]
        #[case("-", 0, 1, "empty after minus")]
        #[case("3.14", 0, 4, "float-like")]
        #[case("0x1F", 0, 4, "hex format")]
        #[case("1e5", 0, 3, "scientific notation")]
        fn test_invalid_integer(
            #[case] input: &str,
            #[case] start: usize,
            #[case] end: usize,
            #[case] _description: &str,
        ) {
            let mut lexer = JasmLexer::new(input);
            let result = lexer.tokenize();

            assert!(
                matches!(
                    &result,
                    Err(LexerError::InvalidNumber(span, _)) if span.start == start && span.end == end
                ),
                "Expected InvalidNumber error for '{}', got {:?}",
                input,
                result
            );
        }

        #[test]
        fn test_invalid_integer_after_valid_token() {
            let mut lexer = JasmLexer::new(".code 999999999999999");
            let result = lexer.tokenize();

            assert!(matches!(
                result,
                Err(LexerError::InvalidNumber(span, _)) if span.start == 6
            ));
        }

        #[test]
        fn test_invalid_number_error_message() {
            let mut lexer = JasmLexer::new("123abc");
            let result = lexer.tokenize();

            if let Err(LexerError::InvalidNumber(_, value)) = result {
                assert_eq!(value, "123abc");
            } else {
                panic!("Expected InvalidNumber error");
            }
        }
    }
}

mod init_clinit {
    use super::*;

    mod success {
        use super::*;

        #[test]
        fn test_init_basic() {
            let mut lexer = JasmLexer::new("<init>");
            let tokens = lexer.tokenize().unwrap();

            assert_eq!(
                tokens,
                vec![
                    JasmToken {
                        kind: JasmTokenKind::Identifier("<init>".to_string()),
                        span: Span::new(0, 6),
                    },
                    JasmToken {
                        kind: JasmTokenKind::Eof,
                        span: Span::new(6, 6),
                    },
                ]
            );
        }

        #[test]
        fn test_clinit_basic() {
            let mut lexer = JasmLexer::new("<clinit>");
            let tokens = lexer.tokenize().unwrap();

            assert_eq!(
                tokens,
                vec![
                    JasmToken {
                        kind: JasmTokenKind::Identifier("<clinit>".to_string()),
                        span: Span::new(0, 8),
                    },
                    JasmToken {
                        kind: JasmTokenKind::Eof,
                        span: Span::new(8, 8),
                    },
                ]
            );
        }

        #[test]
        fn test_init_with_descriptor() {
            let mut lexer = JasmLexer::new("<init>()V");
            let tokens = lexer.tokenize().unwrap();

            assert_eq!(
                tokens,
                vec![
                    JasmToken {
                        kind: JasmTokenKind::Identifier("<init>".to_string()),
                        span: Span::new(0, 6),
                    },
                    JasmToken {
                        kind: JasmTokenKind::OpenParen,
                        span: Span::new(6, 7),
                    },
                    JasmToken {
                        kind: JasmTokenKind::CloseParen,
                        span: Span::new(7, 8),
                    },
                    JasmToken {
                        kind: JasmTokenKind::Identifier("V".to_string()),
                        span: Span::new(8, 9),
                    },
                    JasmToken {
                        kind: JasmTokenKind::Eof,
                        span: Span::new(9, 9),
                    },
                ]
            );
        }

        #[test]
        fn test_init_with_descriptor_and_space_after() {
            let mut lexer = JasmLexer::new("<init> ()V");
            let tokens = lexer.tokenize().unwrap();

            assert_eq!(
                tokens,
                vec![
                    JasmToken {
                        kind: JasmTokenKind::Identifier("<init>".to_string()),
                        span: Span::new(0, 6),
                    },
                    JasmToken {
                        kind: JasmTokenKind::OpenParen,
                        span: Span::new(7, 8),
                    },
                    JasmToken {
                        kind: JasmTokenKind::CloseParen,
                        span: Span::new(8, 9),
                    },
                    JasmToken {
                        kind: JasmTokenKind::Identifier("V".to_string()),
                        span: Span::new(9, 10),
                    },
                    JasmToken {
                        kind: JasmTokenKind::Eof,
                        span: Span::new(10, 10),
                    },
                ]
            );
        }

        #[test]
        fn test_clinit_with_descriptor() {
            let mut lexer = JasmLexer::new("<clinit>()V");
            let tokens = lexer.tokenize().unwrap();

            assert_eq!(
                tokens,
                vec![
                    JasmToken {
                        kind: JasmTokenKind::Identifier("<clinit>".to_string()),
                        span: Span::new(0, 8),
                    },
                    JasmToken {
                        kind: JasmTokenKind::OpenParen,
                        span: Span::new(8, 9),
                    },
                    JasmToken {
                        kind: JasmTokenKind::CloseParen,
                        span: Span::new(9, 10),
                    },
                    JasmToken {
                        kind: JasmTokenKind::Identifier("V".to_string()),
                        span: Span::new(10, 11),
                    },
                    JasmToken {
                        kind: JasmTokenKind::Eof,
                        span: Span::new(11, 11),
                    },
                ]
            );
        }

        #[test]
        fn test_init_in_method_definition() {
            let mut lexer = JasmLexer::new(".method public <init>()V");
            let tokens = lexer.tokenize().unwrap();

            assert_eq!(
                tokens,
                vec![
                    JasmToken {
                        kind: JasmTokenKind::DotMethod,
                        span: Span::new(0, 7),
                    },
                    JasmToken {
                        kind: JasmTokenKind::Public,
                        span: Span::new(8, 14),
                    },
                    JasmToken {
                        kind: JasmTokenKind::Identifier("<init>".to_string()),
                        span: Span::new(15, 21),
                    },
                    JasmToken {
                        kind: JasmTokenKind::OpenParen,
                        span: Span::new(21, 22),
                    },
                    JasmToken {
                        kind: JasmTokenKind::CloseParen,
                        span: Span::new(22, 23),
                    },
                    JasmToken {
                        kind: JasmTokenKind::Identifier("V".to_string()),
                        span: Span::new(23, 24),
                    },
                    JasmToken {
                        kind: JasmTokenKind::Eof,
                        span: Span::new(24, 24),
                    },
                ]
            );
        }

        #[test]
        fn test_clinit_in_method_definition() {
            let mut lexer = JasmLexer::new(".method static <clinit>()V");
            let tokens = lexer.tokenize().unwrap();

            assert_eq!(
                tokens,
                vec![
                    JasmToken {
                        kind: JasmTokenKind::DotMethod,
                        span: Span::new(0, 7),
                    },
                    JasmToken {
                        kind: JasmTokenKind::Static,
                        span: Span::new(8, 14),
                    },
                    JasmToken {
                        kind: JasmTokenKind::Identifier("<clinit>".to_string()),
                        span: Span::new(15, 23),
                    },
                    JasmToken {
                        kind: JasmTokenKind::OpenParen,
                        span: Span::new(23, 24),
                    },
                    JasmToken {
                        kind: JasmTokenKind::CloseParen,
                        span: Span::new(24, 25),
                    },
                    JasmToken {
                        kind: JasmTokenKind::Identifier("V".to_string()),
                        span: Span::new(25, 26),
                    },
                    JasmToken {
                        kind: JasmTokenKind::Eof,
                        span: Span::new(26, 26),
                    },
                ]
            );
        }

        #[test]
        fn test_init_followed_by_other_tokens() {
            let mut lexer = JasmLexer::new("<init> public");
            let tokens = lexer.tokenize().unwrap();

            assert_eq!(
                tokens,
                vec![
                    JasmToken {
                        kind: JasmTokenKind::Identifier("<init>".to_string()),
                        span: Span::new(0, 6),
                    },
                    JasmToken {
                        kind: JasmTokenKind::Public,
                        span: Span::new(7, 13),
                    },
                    JasmToken {
                        kind: JasmTokenKind::Eof,
                        span: Span::new(13, 13),
                    },
                ]
            );
        }

        #[test]
        fn test_init_with_suffix() {
            // <init>V is valid - the >V part becomes identifier
            let mut lexer = JasmLexer::new("<init>V");
            let tokens = lexer.tokenize().unwrap();

            assert_eq!(
                tokens,
                vec![
                    JasmToken {
                        kind: JasmTokenKind::Identifier("<init>V".to_string()),
                        span: Span::new(0, 7),
                    },
                    JasmToken {
                        kind: JasmTokenKind::Eof,
                        span: Span::new(7, 7),
                    },
                ]
            );
        }

        #[test]
        fn test_clinit_with_suffix() {
            let mut lexer = JasmLexer::new("<clinit>V");
            let tokens = lexer.tokenize().unwrap();

            assert_eq!(
                tokens,
                vec![
                    JasmToken {
                        kind: JasmTokenKind::Identifier("<clinit>V".to_string()),
                        span: Span::new(0, 9),
                    },
                    JasmToken {
                        kind: JasmTokenKind::Eof,
                        span: Span::new(9, 9),
                    },
                ]
            );
        }
    }

    mod errors {
        use super::*;

        #[test]
        fn test_invalid_angle_bracket_identifier() {
            let mut lexer = JasmLexer::new("<other>");
            let result = lexer.tokenize();

            assert!(
                matches!(result, Err(LexerError::UnexpectedChar(ref span, '<', _)) if span.start == 0)
            );
        }

        #[test]
        fn test_just_angle_bracket() {
            let mut lexer = JasmLexer::new("<");
            let result = lexer.tokenize();

            assert!(
                matches!(result, Err(LexerError::UnexpectedChar(ref span, '<', _)) if span.start == 0)
            );
        }

        #[test]
        fn test_angle_bracket_after_token() {
            let mut lexer = JasmLexer::new(".class <other>");
            let result = lexer.tokenize();

            assert!(
                matches!(result, Err(LexerError::UnexpectedChar(ref span, '<', _)) if span.start == 7)
            );
        }

        #[test]
        fn test_invalid_angle_bracket_error_message() {
            let mut lexer = JasmLexer::new("<other>");
            let result = lexer.tokenize();

            if let Err(LexerError::UnexpectedChar(_, '<', _)) = result {
                // OK
            } else {
                panic!("Expected UnexpectedChar error for '<'");
            }
        }

        #[test]
        fn test_init_without_closing_bracket() {
            // <init without > is an error
            let mut lexer = JasmLexer::new("<init");
            let result = lexer.tokenize();

            assert!(
                matches!(result, Err(LexerError::UnexpectedChar(ref span, '<', _)) if span.start == 0)
            );
        }

        #[test]
        fn test_clinit_without_closing_bracket() {
            // <clinit without > is an error
            let mut lexer = JasmLexer::new("<clinit");
            let result = lexer.tokenize();

            assert!(
                matches!(result, Err(LexerError::UnexpectedChar(ref span, '<', _)) if span.start == 0)
            );
        }
    }
}

 */
