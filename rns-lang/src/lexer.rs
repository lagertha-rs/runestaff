use crate::token::{JasmToken, JasmTokenKind, Span};
use std::iter::Peekable;
use std::ops::Range;
use std::str::CharIndices;

enum InternalLexerError {
    UnexpectedEof,
    UnexpectedChar(char),
    UnknownToken(String),
    UnterminatedString,
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
                "Multiple-line strings are not supported, make sure to close the string before the end of the line.".to_string()
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
                            '"' => result.push('"'),
                            '\\' => result.push('\\'),
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
            '0'..='9' | '-' => {
                // Handle numbers
                unimplemented!()
            }
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
            _ => {
                return Err(LexerError::UnexpectedChar(
                    start,
                    ch,
                    "TODO: add context".to_string(),
                ));
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
                    "Expected one of the directives: .class, .super, .method, .end, .limit"
                        .to_string()
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
                    "Expected one of the directives: .class, .super, .method, .end, .limit"
                        .to_string()
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
}
