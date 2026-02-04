use crate::token::{JasmToken, JasmTokenKind, Span};
use std::iter::Peekable;
use std::ops::Range;
use std::str::{CharIndices, FromStr};

enum InternalLexerError {
    UnexpectedEof,
    UnexpectedChar(char),
    UnknownToken(String),
    UnterminatedString,
    NotANumber(String),
}

#[derive(Debug, Eq, PartialEq, Clone)]
pub enum LexerError {
    UnexpectedChar(usize, char, String),
    UnknownDirective(Span, String),
    UnexpectedEof(usize, String),
    UnterminatedString(usize),
}

impl LexerError {
    pub fn note(&self) -> String {
        match self {
            LexerError::UnexpectedEof(_, context) => context.clone(),
            LexerError::UnexpectedChar(_, _, context) => context.clone(),
            LexerError::UnterminatedString(_) => {
                "String literal is not terminated before the end of the line or file.".to_string()
            }
            LexerError::UnknownDirective(_, _) => {
                format!("Valid directives are {}", JasmTokenKind::list_directives())
            }
        }
    }

    pub fn as_range(&self) -> Range<usize> {
        match self {
            LexerError::UnknownDirective(span, _) => span.as_range(),
            LexerError::UnexpectedEof(pos, _) => *pos..(*pos + 1),
            LexerError::UnexpectedChar(pos, c, _) => *pos..(*pos + c.len_utf8()),
            LexerError::UnterminatedString(pos) => *pos..(*pos + 1),
        }
    }

    pub fn label(&self) -> String {
        match self {
            LexerError::UnexpectedChar(_, c, _) => {
                format!("Unexpected character '{}'", c.escape_default())
            }
            LexerError::UnknownDirective(_, name) => format!("Unknown directive '{}'", name),
            LexerError::UnexpectedEof(_, _) => "Unexpected end of file".to_string(),
            LexerError::UnterminatedString(_) => {
                "String started here is not terminated".to_string()
            }
        }
    }
}

pub struct JasmLexer<'a> {
    data: Peekable<CharIndices<'a>>,
    byte_pos: usize,
}

impl<'a> JasmLexer<'a> {
    pub fn new(source: &'a str) -> Self {
        Self {
            data: source.char_indices().peekable(),
            byte_pos: 0,
        }
    }

    fn next_char(&mut self) -> Option<char> {
        if let Some((idx, c)) = self.data.next() {
            self.byte_pos = idx + c.len_utf8();
            Some(c)
        } else {
            None
        }
    }

    pub fn skip_whitespaces_and_comments(&mut self) {
        while let Some((_, c)) = self.data.peek() {
            match c {
                ' ' | '\t' | '\r' => {
                    self.next_char();
                }
                ';' => {
                    self.next_char();
                    while let Some((_, c2)) = self.data.peek() {
                        if *c2 != '\n' {
                            self.next_char();
                        } else {
                            break;
                        }
                    }
                }
                _ => break,
            }
        }
    }

    fn read_to_whitespace(&mut self) -> String {
        let mut result = String::new();
        while let Some(&(_, c)) = self.data.peek() {
            if !c.is_whitespace() {
                result.push(c);
                self.next_char();
            } else {
                break;
            }
        }
        result
    }

    fn read_string(&mut self) -> Result<String, InternalLexerError> {
        let mut result = String::new();

        self.next_char(); // consume opening quote

        while let Some(&(_, c)) = self.data.peek() {
            match c {
                '"' => {
                    self.next_char(); // consume closing quote
                    return Ok(result);
                }
                '\n' | '\r' => {
                    return Err(InternalLexerError::UnterminatedString);
                }
                '\\' => {
                    self.next_char(); // consume '\'
                    if let Some(&(_, next_char)) = self.data.peek() {
                        match next_char {
                            'n' => result.push('\n'),
                            't' => result.push('\t'),
                            'r' => result.push('\r'),
                            '0' => result.push('\0'),
                            'b' => result.push('\x08'), // backspace
                            'f' => result.push('\x0C'), // form feed
                            '"' => result.push('"'),
                            '\\' => result.push('\\'),
                            '\n' | '\r' => {
                                return Err(InternalLexerError::UnterminatedString);
                            }
                            _ => result.push(next_char),
                        }
                        self.next_char(); // consume escaped character
                    } else {
                        return Err(InternalLexerError::UnterminatedString);
                    }
                }
                _ => {
                    result.push(c);
                    self.next_char();
                }
            }
        }

        Err(InternalLexerError::UnterminatedString)
    }

    fn read_number(&mut self) -> Result<JasmTokenKind, InternalLexerError> {
        // TODO: implement all number formats and types, right now only integers are supported
        let number_str = self.read_to_whitespace();
        i32::from_str(&number_str)
            .map(JasmTokenKind::Integer)
            .map_err(|_| InternalLexerError::NotANumber(number_str))
    }

    fn handle_directive(&mut self) -> Result<JasmTokenKind, InternalLexerError> {
        self.next_char(); // consume '.'

        let directive = self.read_to_whitespace();
        if directive.is_empty() {
            if let Some(&(_, ch)) = self.data.peek() {
                return Err(InternalLexerError::UnexpectedChar(ch));
            }
            return Err(InternalLexerError::UnexpectedEof);
        }

        JasmTokenKind::from_directive(&directive)
            .ok_or(InternalLexerError::UnknownToken(format!(".{directive}")))
    }

    fn next_token(&mut self) -> Result<JasmToken, LexerError> {
        self.skip_whitespaces_and_comments();

        let start = self.byte_pos;

        let Some(&(_, ch)) = self.data.peek() else {
            return Ok(JasmToken {
                kind: JasmTokenKind::Eof,
                span: Span::new(start, start),
            });
        };

        let kind = match ch {
            '.' => self.handle_directive().map_err(|e| {
                let err_context = format!(
                    "Expected one of the directives: {}",
                    JasmTokenKind::list_directives()
                );
                match e {
                    InternalLexerError::UnexpectedEof => {
                        LexerError::UnexpectedEof(start, err_context)
                    }
                    InternalLexerError::UnknownToken(name) => {
                        LexerError::UnknownDirective(Span::new(start, self.byte_pos), name)
                    }
                    InternalLexerError::UnexpectedChar(c) => {
                        LexerError::UnexpectedChar(self.byte_pos, c, err_context)
                    }
                    _ => unreachable!(),
                }
            })?,
            'a'..='z' | 'A'..='Z' | '_' => {
                let str = self.read_to_whitespace();
                JasmTokenKind::from_identifier(str)
            }
            '0'..='9' | '-' => self.read_number().map_err(|_| todo!())?,
            '"' => JasmTokenKind::StringLiteral(self.read_string().map_err(|e| match e {
                InternalLexerError::UnterminatedString => LexerError::UnterminatedString(start),
                _ => unreachable!(),
            })?),
            '\n' => {
                self.next_char();
                return Ok(JasmToken {
                    kind: JasmTokenKind::Newline,
                    span: Span::new(start, self.byte_pos),
                });
            }
            '[' => {
                self.next_char();
                JasmTokenKind::OpenBracket
            }
            '(' => {
                self.next_char();
                JasmTokenKind::OpenParen
            }
            ')' => {
                self.next_char();
                JasmTokenKind::CloseParen
            }
            _ => {
                let token_str = self.read_to_whitespace();
                if ch == '<' && token_str.starts_with("<init>") {
                    Ok(JasmTokenKind::Identifier(token_str))
                } else {
                    Err(LexerError::UnexpectedChar(
                        start,
                        ch,
                        "TODO: add context".to_string(),
                    ))
                }?
            }
        };

        let end = self.byte_pos;
        Ok(JasmToken {
            kind,
            span: Span::new(start, end),
        })
    }

    pub fn tokenize(&mut self) -> Result<Vec<JasmToken>, LexerError> {
        let mut tokens = Vec::new();

        loop {
            let token = self.next_token()?;
            if let JasmTokenKind::Eof = token.kind {
                tokens.push(token);
                break;
            }
            tokens.push(token);
        }

        Ok(tokens)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    //TODO: test unicode handling
    mod directives {
        use super::*;
        use rstest::rstest;

        #[test]
        fn test_valid_tokenize_directives() {
            const INPUT: &str = ".class .super .method .end .limit";
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
                        kind: JasmTokenKind::DotLimit,
                        span: Span::new(27, 33),
                    },
                    JasmToken {
                        kind: JasmTokenKind::Eof,
                        span: Span::new(33, 33),
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

            assert_eq!(
                tokens,
                Err(LexerError::UnexpectedEof(
                    11,
                    format!(
                        "Expected one of the directives: {}",
                        JasmTokenKind::list_directives()
                    )
                ))
            )
        }

        #[rstest]
        #[case(".class\n    .\n.method", 12, '\n')]
        #[case(".class\n    . .limit\n.method", 12, ' ')]
        #[case(".class\n    .\t.limit\n.method", 12, '\t')]
        #[case(".class\n    .\r.limit\n.method", 12, '\r')]
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
                    pos,
                    c,
                    format!(
                        "Expected one of the directives: {}",
                        JasmTokenKind::list_directives()
                    )
                ))
            )
        }
    }

    mod identifiers_and_keywords {
        use super::*;

        #[test]
        fn test_method_definition() {
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
                        kind: JasmTokenKind::Identifier("main([Ljava/lang/String;)V".to_string()),
                        span: Span::new(22, 48),
                    },
                    JasmToken {
                        kind: JasmTokenKind::Eof,
                        span: Span::new(48, 48),
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

                assert_eq!(result, Err(LexerError::UnterminatedString(0)));
            }

            #[test]
            fn test_backslash_followed_by_literal_carriage_return() {
                // String with backslash followed by carriage return should be treated as unterminated
                let mut lexer = JasmLexer::new("\"test\\\r");
                let result = lexer.tokenize();

                assert_eq!(result, Err(LexerError::UnterminatedString(0)));
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

                assert_eq!(result, Err(LexerError::UnterminatedString(expected_pos)));
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
            // Note: avoiding numbers as they are unimplemented
            let input = r#".class public HelloWorld
.super java/lang/Object

.method public static main([Ljava/lang/String;)V
    .limit stack
    .limit locals
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
                Err(LexerError::UnexpectedChar(p, c, _)) if p == pos && c == ch
            ));
        }

        #[test]
        fn test_unexpected_char_after_valid_token() {
            let mut lexer = JasmLexer::new(".class @");
            let result = lexer.tokenize();

            assert!(matches!(result, Err(LexerError::UnexpectedChar(7, '@', _))));
        }

        #[test]
        fn test_unexpected_char_between_valid_tokens() {
            let mut lexer = JasmLexer::new(".class @ .super");
            let result = lexer.tokenize();

            assert!(matches!(result, Err(LexerError::UnexpectedChar(7, '@', _))));
        }

        #[test]
        fn test_unexpected_char_on_second_line() {
            let mut lexer = JasmLexer::new(".class\n@");
            let result = lexer.tokenize();

            assert!(matches!(result, Err(LexerError::UnexpectedChar(7, '@', _))));
        }

        #[test]
        fn test_unexpected_char_error_message_context() {
            let mut lexer = JasmLexer::new("@");
            let result = lexer.tokenize();

            if let Err(LexerError::UnexpectedChar(_, _, context)) = result {
                assert!(context.contains("TODO")); // Current implementation has "TODO: add context"
            } else {
                panic!("Expected UnexpectedChar error");
            }
        }
    }
}
