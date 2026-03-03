use crate::diagnostic::Diagnostic;
use crate::lexer::error::LexerError;
use crate::token::type_hint::{TypeHint, TypeHintKind};
use crate::token::{RnsToken, Span, Spanned};
use std::iter::Peekable;
use std::str::{CharIndices, FromStr};

mod error;
#[cfg(test)]
mod tests;

pub struct RnsLexer<'a> {
    data: Peekable<CharIndices<'a>>,
    global_pos: usize,
    line_pos: usize,
    line: usize,
}

impl<'a> RnsLexer<'a> {
    // TODO: do I really need an instance?
    pub fn new(source: &'a str) -> Self {
        Self {
            data: source.char_indices().peekable(),
            global_pos: 0,
            line_pos: 0,
            line: 0,
        }
    }

    fn peek_char(&mut self) -> Option<char> {
        self.data.peek().map(|&(_, c)| c)
    }

    fn next_char(&mut self) -> Option<char> {
        if let Some((idx, c)) = self.data.next() {
            if c == '\n' {
                self.line += 1;
                self.line_pos = 0;
            }
            self.global_pos = idx + c.len_utf8();
            self.line_pos += 1;
            Some(c)
        } else {
            None
        }
    }

    pub fn skip_whitespaces_and_comments(&mut self) {
        while let Some(c) = self.peek_char() {
            match c {
                ' ' | '\t' | '\r' => {
                    self.next_char();
                }
                ';' => {
                    self.next_char();
                    while let Some(c2) = self.peek_char() {
                        if c2 != '\n' {
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

    fn is_delimiter(c: char) -> bool {
        c.is_whitespace()
    }

    fn extend_to_delimiter(&mut self, mut start: String) -> String {
        while let Some(c) = self.peek_char() {
            if !Self::is_delimiter(c) {
                start.push(c);
                self.next_char();
            } else {
                break;
            }
        }
        start
    }

    fn read_to_delimiter(&mut self) -> String {
        self.extend_to_delimiter(String::new())
    }

    fn read_string(&mut self, byte_start: usize, col_start: usize) -> Result<String, LexerError> {
        let mut result = String::new();

        self.next_char(); // consume opening quote

        while let Some(c) = self.peek_char() {
            match c {
                '"' => {
                    self.next_char(); // consume closing quote
                    return Ok(result);
                }
                '\n' | '\r' => {
                    return Err(LexerError::UnterminatedString(Span {
                        byte_start,
                        byte_end: byte_start + 1, // TODO: why I put + 1 here? should be cur pos?
                        line: self.line,
                        col_start,
                        col_end: col_start + 1,
                    }));
                }
                '\\' => {
                    self.next_char(); // consume '\'
                    if let Some(next_char) = self.peek_char() {
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
                                return Err(LexerError::UnterminatedString(Span {
                                    byte_start,
                                    byte_end: byte_start + 1, // TODO: why I put + 1 here? should be cur pos?
                                    line: self.line,
                                    col_start,
                                    col_end: col_start + 1,
                                }));
                            }
                            _ => {
                                return Err(LexerError::InvalidEscape(
                                    Span {
                                        byte_start: self.global_pos,
                                        byte_end: self.global_pos + next_char.len_utf8(),
                                        line: self.line,
                                        col_start,
                                        col_end: col_start + 1,
                                    },
                                    next_char,
                                ));
                            }
                        }
                        self.next_char(); // consume escaped character
                    } else {
                        return Err(LexerError::UnterminatedString(Span {
                            byte_start,
                            byte_end: byte_start + 1, // TODO: why I put + 1 here? should be cur pos?
                            line: self.line,
                            col_start,
                            col_end: col_start + 1,
                        }));
                    }
                }
                _ => {
                    result.push(c);
                    self.next_char();
                }
            }
        }

        Err(LexerError::UnterminatedString(Span {
            byte_start,
            byte_end: byte_start + 1, // TODO: why I put + 1 here? should be cur pos?
            line: self.line,
            col_start,
            col_end: col_start + 1,
        }))
    }

    fn read_number(&mut self, byte_start: usize, col_start: usize) -> Result<RnsToken, LexerError> {
        // TODO: implement all number formats and types, right now only integers are supported
        let number_str = self.read_to_delimiter();
        i32::from_str(&number_str)
            .map(|n| {
                RnsToken::Integer(Spanned::new(
                    n,
                    Span {
                        byte_start,
                        byte_end: self.global_pos,
                        line: self.line,
                        col_start,
                        col_end: self.line_pos,
                    },
                ))
            })
            .map_err(|_| {
                LexerError::InvalidNumber(
                    Span {
                        byte_start,
                        byte_end: self.global_pos,
                        line: self.line,
                        col_start,
                        col_end: self.line_pos,
                    },
                    number_str,
                )
            })
    }

    fn handle_directive(
        &mut self,
        byte_start: usize,
        col_pos: usize,
    ) -> Result<RnsToken, LexerError> {
        self.next_char(); // consume '.'

        let directive = self.read_to_delimiter();
        if directive.is_empty() {
            if let Some(ch) = self.peek_char() {
                return Err(LexerError::UnexpectedChar(
                    Span {
                        byte_start: self.global_pos,
                        byte_end: self.global_pos + ch.len_utf8(),
                        line: self.line,
                        col_start: self.line_pos,
                        col_end: self.line_pos + 1,
                    },
                    ch,
                    Some(format!(
                        "Expected one of the directives: {}",
                        RnsToken::list_directives()
                    )),
                ));
            }
            return Err(LexerError::UnexpectedEof(Span {
                byte_start,
                byte_end: byte_start + 1,
                line: self.line,
                col_start: col_pos,
                col_end: col_pos + 1,
            }));
        }

        let span = Span {
            byte_start,
            byte_end: self.global_pos,
            line: self.line,
            col_start: col_pos,
            col_end: self.line_pos,
        };
        RnsToken::from_directive(&directive, span)
            .ok_or(LexerError::UnknownDirective(span, format!(".{directive}")))
    }

    fn expect_identifier(
        &mut self,
        on_err: impl FnOnce(RnsToken) -> LexerError,
    ) -> Result<Spanned<String>, LexerError> {
        let next_token = self.next_token()?;
        match next_token {
            RnsToken::Identifier(spanned) => Ok(spanned),
            other => Err(on_err(other)),
        }
    }

    fn handle_type_hint(
        &mut self,
        byte_start: usize,
        col_start: usize,
    ) -> Result<RnsToken, LexerError> {
        self.next_char(); // consume '@'

        let type_hint_str = self.read_to_delimiter();
        let type_hint_pos = Span {
            byte_start,
            byte_end: self.global_pos,
            line: self.line,
            col_start,
            col_end: self.line_pos,
        };
        let type_hint_kind = TypeHintKind::from_str(&type_hint_str).unwrap();

        self.skip_whitespaces_and_comments();

        let type_hint =
            match type_hint_kind {
                TypeHintKind::Utf8 => TypeHint::Utf8(self.expect_identifier(|t| {
                    LexerError::UnexpectedHintOperand {
                        hint_position: type_hint_pos,
                        operand_token: t,
                        hint_kind: type_hint_kind,
                        operand_order_nbr: 0,
                    }
                })?),
                TypeHintKind::Integer => todo!(),
                TypeHintKind::String => todo!(),
                TypeHintKind::Class => todo!(),
                TypeHintKind::Methodref => {
                    let class = self.expect_identifier(|t| LexerError::UnexpectedHintOperand {
                        hint_position: type_hint_pos,
                        operand_token: t,
                        hint_kind: type_hint_kind.clone(),
                        operand_order_nbr: 0,
                    })?;
                    self.skip_whitespaces_and_comments();
                    let method_name =
                        self.expect_identifier(|t| LexerError::UnexpectedHintOperand {
                            hint_position: type_hint_pos,
                            operand_token: t,
                            hint_kind: type_hint_kind.clone(),
                            operand_order_nbr: 1,
                        })?;
                    self.skip_whitespaces_and_comments();
                    let method_desc =
                        self.expect_identifier(|t| LexerError::UnexpectedHintOperand {
                            hint_position: type_hint_pos,
                            operand_token: t,
                            hint_kind: type_hint_kind,
                            operand_order_nbr: 2,
                        })?;
                    TypeHint::Methodref(class, method_name, method_desc)
                }
                _ => unimplemented!(),
            };

        Ok(RnsToken::Typed(Spanned::new(type_hint, type_hint_pos)))
    }

    fn next_token(&mut self) -> Result<RnsToken, LexerError> {
        self.skip_whitespaces_and_comments();

        let byte_start = self.global_pos;
        let col_start = self.line_pos;

        let Some(ch) = self.peek_char() else {
            return Ok(RnsToken::Eof(Span {
                byte_start,
                byte_end: byte_start,
                line: self.line,
                col_start: self.line_pos,
                col_end: self.line_pos,
            }));
        };

        let token = match ch {
            '.' => self.handle_directive(byte_start, col_start)?,
            '0'..='9' | '-' => self.read_number(byte_start, col_start)?,
            '"' => RnsToken::StringLiteral(Spanned::new(
                self.read_string(byte_start, col_start)?,
                Span {
                    byte_start,
                    byte_end: self.global_pos,
                    line: self.line,
                    col_start,
                    col_end: self.line_pos,
                },
            )),
            '#' => {
                self.next_char();
                if let Some(next) = self.peek_char()
                    && next == '"'
                {
                    RnsToken::Identifier(Spanned::new(
                        self.read_string(byte_start, col_start)?,
                        Span {
                            byte_start,
                            byte_end: self.global_pos,
                            line: self.line,
                            col_start,
                            col_end: self.line_pos,
                        },
                    ))
                } else {
                    RnsToken::Identifier(Spanned::new(
                        self.extend_to_delimiter(String::new()),
                        Span {
                            byte_start,
                            byte_end: self.global_pos,
                            line: self.line,
                            col_start,
                            col_end: self.line_pos,
                        },
                    ))
                }
            }
            '@' => self.handle_type_hint(byte_start, col_start)?,
            '\n' => {
                self.next_char();
                RnsToken::Newline(Span {
                    byte_start,
                    byte_end: self.global_pos,
                    line: self.line - 1, // TODO: is this correct?
                    col_start,
                    col_end: self.line_pos,
                })
            }
            _ => {
                let str = self.read_to_delimiter();
                RnsToken::from_identifier(
                    str,
                    Span {
                        byte_start,
                        byte_end: self.global_pos,
                        line: self.line,
                        col_start,
                        col_end: self.line_pos,
                    },
                )
            }
        };
        Ok(token)
    }

    pub fn tokenize(&mut self) -> Result<Vec<RnsToken>, Diagnostic> {
        let mut tokens = Vec::new();

        loop {
            let token = self.next_token()?;
            if let RnsToken::Eof(_) = token {
                tokens.push(token);
                break;
            }
            tokens.push(token);
        }

        Ok(tokens)
    }
}
