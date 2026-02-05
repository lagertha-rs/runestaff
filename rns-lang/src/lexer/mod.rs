use crate::token::{JasmToken, JasmTokenKind, Span};
use std::iter::Peekable;
use std::ops::Range;
use std::str::{CharIndices, FromStr};

// TODO: Probably replace by categorized enums to avoid unreachable()
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
    InvalidNumber(Span, String),
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
            LexerError::InvalidNumber(_, _) => "Expected a valid integer number.".to_string(),
        }
    }

    pub fn as_range(&self) -> Range<usize> {
        match self {
            LexerError::UnknownDirective(span, _) => span.as_range(),
            LexerError::UnexpectedEof(pos, _) => *pos..(*pos + 1),
            LexerError::UnexpectedChar(pos, c, _) => *pos..(*pos + c.len_utf8()),
            LexerError::UnterminatedString(pos) => *pos..(*pos + 1),
            LexerError::InvalidNumber(span, _) => span.as_range(),
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
            LexerError::InvalidNumber(_, value) => {
                format!("'{}' is not a valid integer", value)
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

    fn is_delimiter(c: char) -> bool {
        c.is_whitespace() || matches!(c, '(' | ')' | '[')
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
        let number_str = self.read_to_delimiter();
        i32::from_str(&number_str)
            .map(JasmTokenKind::Integer)
            .map_err(|_| InternalLexerError::NotANumber(number_str))
    }

    fn handle_directive(&mut self) -> Result<JasmTokenKind, InternalLexerError> {
        self.next_char(); // consume '.'

        let directive = self.read_to_delimiter();
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
                let str = self.read_to_delimiter();
                JasmTokenKind::from_identifier(str)
            }
            '0'..='9' | '-' => self.read_number().map_err(|e| match e {
                InternalLexerError::NotANumber(value) => {
                    LexerError::InvalidNumber(Span::new(start, self.byte_pos), value)
                }
                _ => unreachable!(),
            })?,
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
            '<' => {
                let token_str = self.read_to_delimiter();
                if token_str.starts_with("<init>") || token_str.starts_with("<clinit>") {
                    JasmTokenKind::Identifier(token_str)
                } else {
                    return Err(LexerError::UnexpectedChar(
                        start,
                        ch,
                        "Only <init> and <clinit> are valid identifiers starting with '<'"
                            .to_string(),
                    ));
                }
            }
            _ => {
                return Err(LexerError::UnexpectedChar(
                    start,
                    ch,
                    "Unexpected character".to_string(),
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
mod tests;
