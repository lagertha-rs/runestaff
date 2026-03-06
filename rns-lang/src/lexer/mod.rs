use crate::diagnostic::Diagnostic;
use crate::lexer::error::LexerError;
use crate::token::type_hint::{TypeHint, TypeHintKind};
use crate::token::{RnsToken, Span, Spanned};
use std::str::FromStr;

mod error;
#[cfg(test)]
mod snapshot_tests;
#[cfg(test)]
mod tests;

pub struct RnsLexer<'a> {
    source: &'a str,
    byte_pos: usize,
    col_pos: usize,
    line: usize,
}

impl<'a> RnsLexer<'a> {
    // TODO: do I really need an instance?
    pub fn new(source: &'a str) -> Self {
        Self {
            source,
            byte_pos: 0,
            col_pos: 0,
            line: 0,
        }
    }

    fn peek_char(&mut self) -> Option<char> {
        self.source[self.byte_pos..].chars().next()
    }

    fn peek_char_at(&mut self, offset: usize) -> Option<char> {
        self.source[self.byte_pos..].chars().nth(offset)
    }

    fn next_char(&mut self) -> Option<char> {
        if let Some(c) = self.peek_char() {
            self.byte_pos += c.len_utf8();
            if c == '\n' {
                self.line += 1;
                self.col_pos = 0;
            } else {
                self.col_pos += 1;
            }
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

    fn span_for_current_position(&self, byte_start: usize, col_start: usize) -> Span {
        Span {
            byte_start,
            byte_end: self.byte_pos,
            line: self.line,
            col_start,
            col_end: self.col_pos,
        }
    }

    fn span_for_eof(&self, byte_start: usize, col_start: usize) -> Span {
        Span {
            byte_start,
            byte_end: byte_start,
            line: self.line,
            col_start,
            col_end: col_start,
        }
    }

    fn read_string(
        &mut self,
        byte_start: usize,
        col_start: usize,
    ) -> Result<Spanned<String>, LexerError> {
        let mut result = String::new();

        self.next_char(); // consume opening quote

        while let Some(c) = self.peek_char() {
            match c {
                '"' => {
                    self.next_char(); // consume closing quote
                    return Ok(Spanned::new(
                        result,
                        Span {
                            byte_start,
                            byte_end: self.byte_pos,
                            line: self.line,
                            col_start,
                            col_end: self.col_pos,
                        },
                    ));
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
                                        byte_start: self.byte_pos,
                                        byte_end: self.byte_pos + next_char.len_utf8(),
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

    fn read_number(&mut self) -> Result<RnsToken, LexerError> {
        // TODO: implement all number formats and types, right now only integers are supported
        // TODO: consider supporting underscore/hex/binary literals in the future
        let byte_start = self.byte_pos;
        let col_start = self.col_pos;

        let number_str = self.read_to_delimiter();
        let position = self.span_for_current_position(byte_start, col_start);
        i32::from_str(&number_str)
            .map(|n| RnsToken::Integer(Spanned::new(n, position)))
            .map_err(|_| LexerError::InvalidInteger(position, number_str))
    }

    fn handle_directive(&mut self) -> Result<RnsToken, LexerError> {
        let byte_start = self.byte_pos;
        let col_start = self.col_pos;

        self.next_char(); // consume '.'

        let directive = self.read_to_delimiter();
        if directive.is_empty() {
            // TODO: test '.' with and without trailing whitespace
            return Ok(RnsToken::Identifier(Spanned::new(
                ".".to_string(),
                Span {
                    byte_start,
                    byte_end: self.byte_pos,
                    line: self.line,
                    col_start,
                    col_end: self.col_pos,
                },
            )));
        }

        let span = Span {
            byte_start,
            byte_end: self.byte_pos,
            line: self.line,
            col_start,
            col_end: self.col_pos,
        };
        RnsToken::from_directive(&directive, span)
            .ok_or(LexerError::UnknownDirective(span, format!(".{directive}")))
    }

    fn expect_identifier(
        &mut self,
        on_err: impl FnOnce(RnsToken) -> LexerError,
    ) -> Result<Spanned<String>, Diagnostic> {
        let next_token = self.next_token()?;
        match next_token {
            RnsToken::Identifier(spanned) => Ok(spanned),
            other => Err(on_err(other).into()),
        }
    }

    fn handle_type_hint(&mut self) -> Result<RnsToken, Diagnostic> {
        let byte_start = self.byte_pos;
        let col_start = self.col_pos;

        self.next_char(); // consume '@'

        let type_hint_str = self.read_to_delimiter();
        if type_hint_str.is_empty() {
            // TODO: test '@'
            return Ok(RnsToken::Identifier(Spanned::new(
                "@".to_string(),
                Span {
                    byte_start,
                    byte_end: self.byte_pos,
                    line: self.line,
                    col_start,
                    col_end: self.col_pos,
                },
            )));
        }
        let type_hint_pos = Span {
            byte_start,
            byte_end: self.byte_pos,
            line: self.line,
            col_start,
            col_end: self.col_pos,
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

    fn next_token(&mut self) -> Result<RnsToken, Diagnostic> {
        self.skip_whitespaces_and_comments();

        let byte_start = self.byte_pos;
        let col_start = self.col_pos;

        let Some(ch) = self.peek_char() else {
            return Ok(RnsToken::Eof(self.span_for_eof(byte_start, col_start)));
        };

        let token = match ch {
            '.' => self.handle_directive()?,
            '0'..='9' => self.read_number()?,
            '-' => {
                // TODO: test minus only
                let is_digit_after = self.peek_char_at(1).is_some_and(|c| c.is_ascii_digit());
                if is_digit_after {
                    self.read_number()?
                } else {
                    let str = self.read_to_delimiter();
                    RnsToken::from_identifier(
                        str,
                        self.span_for_current_position(byte_start, col_start),
                    )
                }
            }
            '"' => RnsToken::StringLiteral(self.read_string(byte_start, col_start)?),
            '#' => {
                self.next_char();
                if let Some(next) = self.peek_char()
                    && next == '"'
                {
                    RnsToken::Identifier(self.read_string(byte_start, col_start)?)
                } else {
                    RnsToken::Identifier(Spanned::new(
                        self.extend_to_delimiter(String::new()),
                        self.span_for_current_position(byte_start, col_start),
                    ))
                }
            }
            '@' => self.handle_type_hint()?,
            '\n' => {
                self.next_char();
                RnsToken::Newline(Span {
                    byte_start,
                    byte_end: self.byte_pos,
                    line: self.line - 1,
                    col_start,
                    col_end: col_start + 1,
                })
            }
            _ => {
                let str = self.read_to_delimiter();
                RnsToken::from_identifier(
                    str,
                    self.span_for_current_position(byte_start, col_start),
                )
            }
        };
        Ok(token)
    }

    fn skip_to_end_of_line(&mut self) {
        while let Some(c) = self.peek_char() {
            if c == '\n' {
                break;
            }
            self.next_char();
        }
    }

    pub fn tokenize(&mut self) -> (Vec<RnsToken>, Vec<Diagnostic>) {
        let mut tokens = Vec::new();
        let mut diagnostics = Vec::new();

        loop {
            match self.next_token() {
                Ok(token) => {
                    let is_eof = matches!(token, RnsToken::Eof(_));
                    // TODO: can I skip multiple newlines in a row? do I need to keep them all?
                    tokens.push(token);
                    if is_eof {
                        break;
                    }
                }
                Err(e) => {
                    diagnostics.push(e.into());
                    self.skip_to_end_of_line();
                }
            }
        }

        (tokens, diagnostics)
    }
}
