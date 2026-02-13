use crate::error::JasmError;
use crate::lexer::error::LexerError;
use crate::token::{JasmToken, JasmTokenKind, Span};
use std::iter::Peekable;
use std::str::{CharIndices, FromStr};

mod error;
#[cfg(test)]
mod tests;

pub struct JasmLexer<'a> {
    data: Peekable<CharIndices<'a>>,
    byte_pos: usize,
}

impl<'a> JasmLexer<'a> {
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

    fn read_number(&mut self, start: usize) -> Result<JasmTokenKind, LexerError> {
        // TODO: implement all number formats and types, right now only integers are supported
        let number_str = self.read_to_delimiter();
        i32::from_str(&number_str)
            .map(JasmTokenKind::Integer)
            .map_err(|_| LexerError::InvalidNumber(Span::new(start, self.byte_pos), number_str))
    }

    fn handle_directive(&mut self, start: usize) -> Result<JasmTokenKind, LexerError> {
        self.next_char(); // consume '.'

        let directive = self.read_to_delimiter();
        if directive.is_empty() {
            if let Some(&(_, ch)) = self.data.peek() {
                return Err(LexerError::UnexpectedChar(
                    Span::new(self.byte_pos, self.byte_pos + ch.len_utf8()),
                    ch,
                    Some(format!(
                        "Expected one of the directives: {}",
                        JasmTokenKind::list_directives()
                    )),
                ));
            }
            return Err(LexerError::UnexpectedEof(Span::new(start, start + 1)));
        }

        JasmTokenKind::from_directive(&directive).ok_or(LexerError::UnknownDirective(
            Span::new(start, self.byte_pos),
            format!(".{directive}"),
        ))
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
            '.' => self.handle_directive(start)?,
            'a'..='z' | 'A'..='Z' | '_' | '(' => {
                // TODO: out identifiers can have forbidden characters, but for now we just read until
                // the next whitespace, which is good enough for most cases. In the future we might want
                // to implement more specific rules for identifiers and method descriptors.
                let str = self.read_to_delimiter();
                JasmTokenKind::from_identifier(str)
            }
            '0'..='9' | '-' => self.read_number(start)?,
            '"' => JasmTokenKind::StringLiteral(self.read_string(start)?),
            '\n' => {
                self.next_char();
                return Ok(JasmToken {
                    kind: JasmTokenKind::Newline,
                    span: Span::new(start, self.byte_pos),
                });
            }
            '<' => {
                let token_str = self.read_to_delimiter();
                if token_str.starts_with("<init>") || token_str.starts_with("<clinit>") {
                    JasmTokenKind::Identifier(token_str)
                } else {
                    return Err(LexerError::UnexpectedChar(
                        Span::new(start, start + ch.len_utf8()),
                        ch,
                        None,
                    ));
                }
            }
            _ => {
                return Err(LexerError::UnexpectedChar(
                    Span::new(start, start + ch.len_utf8()),
                    ch,
                    None,
                ));
            }
        };

        let end = self.byte_pos;
        Ok(JasmToken {
            kind,
            span: Span::new(start, end),
        })
    }

    pub fn tokenize(&mut self) -> Result<Vec<JasmToken>, JasmError> {
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
