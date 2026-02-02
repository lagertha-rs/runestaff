use crate::token::{JasmToken, JasmTokenKind, Span};
use std::iter::Peekable;
use std::ops::Range;
use std::str::CharIndices;

enum InternalLexerError {
    UnexpectedEof,
    UnexpectedChar(char),
    UnknownToken(String),
}

#[derive(Debug, Eq, PartialEq, Clone)]
pub enum LexerError {
    UnexpectedChar(usize, char, String),
    UnknownDirective(Span, String),
    UnexpectedEof(usize, String),
}

impl LexerError {
    pub fn note(&self) -> String {
        match self {
            LexerError::UnexpectedEof(_, context) => context.clone(),
            LexerError::UnexpectedChar(_, _, context) => context.clone(),
            LexerError::UnknownDirective(_, _) => format!(
                "Valid directives are {}",
                JasmTokenKind::all_directives_as_comma_separated_string()
            ),
        }
    }

    pub fn as_range(&self) -> Range<usize> {
        match self {
            LexerError::UnknownDirective(span, _) => span.as_range(),
            LexerError::UnexpectedEof(pos, _) => *pos..*pos, // TODO: verify
            LexerError::UnexpectedChar(pos, c, _) => *pos..(*pos + c.len_utf8()),
        }
    }

    pub fn label(&self) -> String {
        match self {
            LexerError::UnexpectedChar(_, c, _) => {
                format!("Unexpected character '{}'", c.escape_default())
            }
            LexerError::UnknownDirective(_, name) => format!("Unknown directive '{}'", name),
            LexerError::UnexpectedEof(_, _) => "Unexpected end of file".to_string(),
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
            '.' => self.handle_directive().map_err(|e| {
                let err_context = format!(
                    "Expected one of the directives: {}",
                    JasmTokenKind::all_directives_as_comma_separated_string()
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

        #[test]
        fn test_tokenize_unknown_directive() {
            const INPUT: &str = ".class\n    .unknown\n.method";
            let mut lexer = JasmLexer::new(INPUT);
            let tokens = lexer.tokenize();

            assert_eq!(
                tokens,
                Err(LexerError::UnknownDirective(
                    Span::new(11, 19),
                    ".unknown".to_string()
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
}
