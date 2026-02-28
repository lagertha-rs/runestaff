use crate::diagnostic::Diagnostic;
use crate::lexer::error::LexerError;
use crate::token::{RnsToken, RnsTokenKind, Span};
use std::iter::Peekable;
use std::str::{CharIndices, FromStr};

mod error;
#[cfg(test)]
mod tests;

pub struct RnsLexer<'a> {
    data: Peekable<CharIndices<'a>>,
    byte_pos: usize,
}

impl<'a> RnsLexer<'a> {
    // TODO: do I really need an instance?
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

    fn is_delimiter(c: char) -> bool {
        c.is_whitespace()
    }

    fn read_to_delimiter(&mut self) -> String {
        let mut result = String::new();
        while let Some(&(_, c)) = self.data.peek() {
            if !Self::is_delimiter(c) {
                result.push(c);
                self.next_char();
            } else {
                break;
            }
        }
        result
    }

    fn read_string(&mut self, start: usize) -> Result<String, LexerError> {
        let mut result = String::new();

        self.next_char(); // consume opening quote

        while let Some(&(_, c)) = self.data.peek() {
            match c {
                '"' => {
                    self.next_char(); // consume closing quote
                    return Ok(result);
                }
                '\n' | '\r' => {
                    return Err(LexerError::UnterminatedString(Span::new(start, start + 1)));
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
                                return Err(LexerError::UnterminatedString(Span::new(
                                    start,
                                    start + 1,
                                )));
                            }
                            _ => {
                                return Err(LexerError::InvalidEscape(
                                    Span::new(self.byte_pos, self.byte_pos + next_char.len_utf8()),
                                    next_char,
                                ));
                            }
                        }
                        self.next_char(); // consume escaped character
                    } else {
                        return Err(LexerError::UnterminatedString(Span::new(start, start + 1)));
                    }
                }
                _ => {
                    result.push(c);
                    self.next_char();
                }
            }
        }

        Err(LexerError::UnterminatedString(Span::new(start, start + 1)))
    }

    fn read_number(&mut self, start: usize) -> Result<RnsTokenKind, LexerError> {
        // TODO: implement all number formats and types, right now only integers are supported
        let number_str = self.read_to_delimiter();
        i32::from_str(&number_str)
            .map(RnsTokenKind::Integer)
            .map_err(|_| LexerError::InvalidNumber(Span::new(start, self.byte_pos), number_str))
    }

    fn handle_directive(&mut self, start: usize) -> Result<RnsTokenKind, LexerError> {
        self.next_char(); // consume '.'

        let directive = self.read_to_delimiter();
        if directive.is_empty() {
            if let Some(&(_, ch)) = self.data.peek() {
                return Err(LexerError::UnexpectedChar(
                    Span::new(self.byte_pos, self.byte_pos + ch.len_utf8()),
                    ch,
                    Some(format!(
                        "Expected one of the directives: {}",
                        RnsTokenKind::list_directives()
                    )),
                ));
            }
            return Err(LexerError::UnexpectedEof(Span::new(start, start + 1)));
        }

        RnsTokenKind::from_directive(&directive).ok_or(LexerError::UnknownDirective(
            Span::new(start, self.byte_pos),
            format!(".{directive}"),
        ))
    }

    fn next_token(&mut self) -> Result<RnsToken, LexerError> {
        self.skip_whitespaces_and_comments();

        let start = self.byte_pos;

        let Some(&(_, ch)) = self.data.peek() else {
            return Ok(RnsToken {
                kind: RnsTokenKind::Eof,
                span: Span::new(start, start),
            });
        };

        let kind = match ch {
            '.' => self.handle_directive(start)?,
            '0'..='9' | '-' => self.read_number(start)?,
            '"' => RnsTokenKind::StringLiteral(self.read_string(start)?),
            '\n' => {
                self.next_char();
                return Ok(RnsToken {
                    kind: RnsTokenKind::Newline,
                    span: Span::new(start, self.byte_pos),
                });
            }
            _ => {
                let str = self.read_to_delimiter();
                RnsTokenKind::from_identifier(str)
            }
        };

        let end = self.byte_pos;
        Ok(RnsToken {
            kind,
            span: Span::new(start, end),
        })
    }

    pub fn tokenize(&mut self) -> Result<Vec<RnsToken>, Diagnostic> {
        let mut tokens = Vec::new();

        loop {
            let token = self.next_token()?;
            if let RnsTokenKind::Eof = token.kind {
                tokens.push(token);
                break;
            }
            tokens.push(token);
        }

        Ok(tokens)
    }
}
