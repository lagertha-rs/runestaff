use crate::cursor::Cursor;
use crate::token::{JasmToken, JasmTokenKind, Span};

enum InternalLexerError {
    UnexpectedEof,
    UnexpectedChar(char),
    UnknownToken(String),
}

#[derive(Debug, Eq, PartialEq, Clone)]
pub enum LexerError {
    UnexpectedChar(char, usize, usize), // char, line, column
    UnknownDirective(Span, String),     // name, line, column
    UnterminatedString(usize),          // line
    InvalidEscape(char, usize),         // char, line
    InvalidNumber(String, usize),       // value, line
    UnexpectedEof(usize, usize),        // line, column
}

impl std::fmt::Display for LexerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LexerError::UnexpectedChar(ch, line, col) => write!(
                f,
                "unexpected character '{}' at line {}, column {}",
                ch, line, col
            ),
            LexerError::UnknownDirective(span, name) => write!(
                f,
                "unknown directive '{}' at line {}, column {}",
                name, span.line, span.start
            ),
            LexerError::UnterminatedString(line) => {
                write!(f, "unterminated string literal at line {}", line)
            }
            LexerError::InvalidEscape(ch, line) => {
                write!(f, "invalid escape sequence '\\{}' at line {}", ch, line)
            }
            LexerError::InvalidNumber(val, line) => {
                write!(f, "invalid number '{}' at line {}", val, line)
            }
            LexerError::UnexpectedEof(line, column) => {
                write!(
                    f,
                    "unexpected end of file at line {}, column {}",
                    line, column
                )
            }
        }
    }
}

pub struct JasmLexer<'a> {
    source: &'a str,
    cursor: Cursor<'a>,
}

impl<'a> JasmLexer<'a> {
    pub fn new(source: &'a str) -> Self {
        Self {
            source,
            cursor: Cursor::new(source.chars().peekable()),
        }
    }

    fn handle_directive(&mut self) -> Result<JasmTokenKind, InternalLexerError> {
        self.cursor.next_char(); // consume '.'

        let directive = self.cursor.next_string_while(|c| !c.is_whitespace());
        if directive.is_empty() {
            if let Some(ch) = self.cursor.peek() {
                return Err(InternalLexerError::UnexpectedChar(ch));
            }
            return Err(InternalLexerError::UnexpectedEof);
        }

        JasmTokenKind::try_directive(&directive)
            .ok_or(InternalLexerError::UnknownToken(format!(".{directive}")))
    }

    fn next_token(&mut self) -> Result<JasmToken, LexerError> {
        self.cursor.skip_whitespaces_and_comments();

        let start = self.cursor.current_column_nbr();
        let line = self.cursor.current_line_nbr();

        let Some(ch) = self.cursor.peek() else {
            return Ok(JasmToken {
                kind: JasmTokenKind::Eof,
                span: Span::new(start, start, line),
            });
        };

        let kind = match ch {
            '.' => self.handle_directive().map_err(|e| match e {
                InternalLexerError::UnexpectedEof => {
                    LexerError::UnexpectedEof(line, self.cursor.current_column_nbr())
                }
                InternalLexerError::UnknownToken(name) => LexerError::UnknownDirective(
                    Span::new(start, start + name.len() + 1, line),
                    name,
                ),
                InternalLexerError::UnexpectedChar(c) => {
                    LexerError::UnexpectedChar(c, line, self.cursor.current_column_nbr())
                }
            })?,
            'a'..='z' | 'A'..='Z' | '_' => {
                // Handle identifiers and keywords
                unimplemented!()
            }
            '0'..='9' => {
                // Handle numbers
                unimplemented!()
            }
            '"' => {
                // Handle string literals
                unimplemented!()
            }
            '\n' => {
                self.cursor.next_char();
                return Ok(JasmToken {
                    kind: JasmTokenKind::Newline,
                    span: Span::new(start, start, line),
                });
            }
            _ => {
                return Err(LexerError::UnexpectedChar(
                    ch,
                    line,
                    self.cursor.current_column_nbr(),
                ));
            }
        };

        let end = self.cursor.current_column_nbr();
        Ok(JasmToken {
            kind,
            span: Span::new(start, end, line),
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
                        span: Span::new(0, 6, 1),
                    },
                    JasmToken {
                        kind: JasmTokenKind::DotSuper,
                        span: Span::new(7, 13, 1),
                    },
                    JasmToken {
                        kind: JasmTokenKind::DotMethod,
                        span: Span::new(14, 21, 1),
                    },
                    JasmToken {
                        kind: JasmTokenKind::DotEnd,
                        span: Span::new(22, 26, 1),
                    },
                    JasmToken {
                        kind: JasmTokenKind::DotLimit,
                        span: Span::new(27, 33, 1),
                    },
                    JasmToken {
                        kind: JasmTokenKind::Eof,
                        span: Span::new(33, 33, 1),
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
                        span: Span::new(1, 1, 1),
                    },
                    JasmToken {
                        kind: JasmTokenKind::DotClass,
                        span: Span::new(4, 10, 2),
                    },
                    JasmToken {
                        kind: JasmTokenKind::DotSuper,
                        span: Span::new(13, 19, 2),
                    },
                    JasmToken {
                        kind: JasmTokenKind::Newline,
                        span: Span::new(20, 20, 2),
                    },
                    JasmToken {
                        kind: JasmTokenKind::DotMethod,
                        span: Span::new(1, 8, 3),
                    },
                    JasmToken {
                        kind: JasmTokenKind::Newline,
                        span: Span::new(10, 10, 3),
                    },
                    JasmToken {
                        kind: JasmTokenKind::DotEnd,
                        span: Span::new(1, 5, 4),
                    },
                    JasmToken {
                        kind: JasmTokenKind::Newline,
                        span: Span::new(7, 7, 4),
                    },
                    JasmToken {
                        kind: JasmTokenKind::Eof,
                        span: Span::new(1, 1, 5),
                    },
                ]
            )
        }

        #[test]
        fn test_tokenize_unknown_directive() {
            const INPUT: &str = ".class\n    .unknown\n.method";
            let mut lexer = JasmLexer::new(INPUT);
            let tokens = lexer.tokenize();

            assert_eq!(
                tokens,
                Err(LexerError::UnknownDirective(
                    Span::new(4, 13, 2),
                    ".unknown".to_string()
                ))
            )
        }

        #[test]
        fn test_tokenize_eof_directive() {
            const INPUT: &str = ".class\n    .";
            let mut lexer = JasmLexer::new(INPUT);
            let tokens = lexer.tokenize();

            assert_eq!(tokens, Err(LexerError::UnexpectedEof(2, 5)))
        }

        #[rstest]
        #[case(".class\n    .\n.method", 2, 5, '\n')]
        #[case(".class\n    . .limit\n.method", 2, 5, ' ')]
        #[case(".class\n    .\t.limit\n.method", 2, 5, '\t')]
        #[case(".class\n    .\r.limit\n.method", 2, 5, '\r')]
        fn test_tokenize_unexpected_char_directive(
            #[case] input: &str,
            #[case] line: usize,
            #[case] column: usize,
            #[case] c: char,
        ) {
            let mut lexer = JasmLexer::new(input);
            let tokens = lexer.tokenize();

            assert_eq!(tokens, Err(LexerError::UnexpectedChar(c, line, column)))
        }
    }
}
