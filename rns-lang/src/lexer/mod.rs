use crate::diagnostic::Diagnostic;
use crate::lexer::error::LexerError;
use crate::token::{RnsToken, Span, Spanned};
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

    fn peek_char(&mut self) -> Option<char> {
        self.data.peek().map(|&(_, c)| c)
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

    fn read_string(&mut self, start: usize) -> Result<String, LexerError> {
        let mut result = String::new();

        self.next_char(); // consume opening quote

        while let Some(c) = self.peek_char() {
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

    fn read_number(&mut self, start: usize) -> Result<RnsToken, LexerError> {
        // TODO: implement all number formats and types, right now only integers are supported
        let number_str = self.read_to_delimiter();
        let number_end_pos = self.byte_pos;
        i32::from_str(&number_str)
            .map(|n| RnsToken::Integer(Spanned::new(n, Span::new(start, number_end_pos))))
            .map_err(|_| LexerError::InvalidNumber(Span::new(start, number_end_pos), number_str))
    }

    fn handle_directive(&mut self, start: usize) -> Result<RnsToken, LexerError> {
        self.next_char(); // consume '.'

        let directive = self.read_to_delimiter();
        if directive.is_empty() {
            if let Some(ch) = self.peek_char() {
                return Err(LexerError::UnexpectedChar(
                    Span::new(self.byte_pos, self.byte_pos + ch.len_utf8()),
                    ch,
                    Some(format!(
                        "Expected one of the directives: {}",
                        RnsToken::list_directives()
                    )),
                ));
            }
            return Err(LexerError::UnexpectedEof(Span::new(start, start + 1)));
        }

        let span = Span::new(start, self.byte_pos);
        RnsToken::from_directive(&directive, span)
            .ok_or(LexerError::UnknownDirective(span, format!(".{directive}")))
    }

    fn handle_type_hint(&mut self, start: usize) -> Result<RnsToken, LexerError> {
        self.next_char(); // consume '@'

        let type_hint = self.read_to_delimiter();

        todo!()
    }

    fn next_token(&mut self) -> Result<RnsToken, LexerError> {
        self.skip_whitespaces_and_comments();

        let start = self.byte_pos;

        let Some(ch) = self.peek_char() else {
            return Ok(RnsToken::Eof(Span::new(start, start)));
        };

        let token = match ch {
            '.' => self.handle_directive(start)?,
            '0'..='9' | '-' => self.read_number(start)?,
            '"' => RnsToken::StringLiteral(Spanned::new(
                self.read_string(start)?,
                Span::new(start, self.byte_pos),
            )),
            '#' => {
                self.next_char();
                if let Some(next) = self.peek_char()
                    && next == '"'
                {
                    RnsToken::Identifier(Spanned::new(
                        self.read_string(start)?,
                        Span::new(start, self.byte_pos),
                    ))
                } else {
                    RnsToken::Identifier(Spanned::new(
                        self.extend_to_delimiter(String::new()),
                        Span::new(start, self.byte_pos),
                    ))
                }
            }
            '@' => self.handle_type_hint(start)?,
            '\n' => {
                self.next_char();
                RnsToken::Newline(Span::new(start, self.byte_pos))
            }
            _ => {
                let str = self.read_to_delimiter();
                RnsToken::from_identifier(str, Span::new(start, self.byte_pos))
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
