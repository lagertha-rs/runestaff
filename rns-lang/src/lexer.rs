use crate::token::{JasmToken, JasmTokenKind, Span};
use std::iter::Peekable;
use std::str::CharIndices;

enum InternalLexerError {
    UnexpectedEof,
    UnexpectedChar(char),
    UnknownToken(String),
}

#[derive(Debug, Eq, PartialEq, Clone)]
pub enum LexerError {
    UnexpectedChar(usize, char),
    UnknownDirective(Span, String),
    UnexpectedEof(usize),
}

impl LexerError {
    pub fn message(&self) -> &str {
        match self {
            LexerError::UnexpectedChar(_, _) => "unexpected character",
            LexerError::UnknownDirective(_, _) => "unknown directive",
            LexerError::UnexpectedEof(_) => "unexpected end of file",
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

    fn take_until_whitespace(&mut self) -> String {
        let mut result = String::new();
        while let Some((_, c)) = self.data.peek() {
            if !c.is_whitespace() {
                result.push(*c);
                self.next_char();
            } else {
                break;
            }
        }
        result
    }

    fn handle_directive(&mut self) -> Result<JasmTokenKind, InternalLexerError> {
        self.next_char(); // consume '.'

        let directive = self.take_until_whitespace();
        if directive.is_empty() {
            if let Some(&(_, ch)) = self.data.peek() {
                return Err(InternalLexerError::UnexpectedChar(ch));
            }
            return Err(InternalLexerError::UnexpectedEof);
        }

        JasmTokenKind::try_directive(&directive)
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
            '.' => self.handle_directive().map_err(|e| match e {
                InternalLexerError::UnexpectedEof => LexerError::UnexpectedEof(start),
                InternalLexerError::UnknownToken(name) => {
                    LexerError::UnknownDirective(Span::new(start, self.byte_pos), name)
                }
                InternalLexerError::UnexpectedChar(c) => LexerError::UnexpectedChar(start, c),
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
                self.next_char();
                return Ok(JasmToken {
                    kind: JasmTokenKind::Newline,
                    span: Span::new(start, start),
                });
            }
            _ => {
                return Err(LexerError::UnexpectedChar(start, ch));
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

/*
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
 */
