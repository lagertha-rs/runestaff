use crate::error::{JasmDiagnostic, JasmError};
use crate::token::{JasmTokenKind, Span};
use std::ops::Range;

#[derive(Debug, Eq, PartialEq, Clone)]
pub(super) enum LexerError {
    UnexpectedChar(Span, char, Option<String>),
    UnknownDirective(Span, String),
    UnexpectedEof(Span),
    UnterminatedString(Span),
    InvalidEscape(Span, char),
    InvalidNumber(Span, String),
}

impl LexerError {
    pub fn message(&self) -> Option<String> {
        let msg = match self {
            LexerError::UnexpectedChar(_, _, _) => "unexpected character",
            LexerError::UnknownDirective(_, _) => "unknown directive",
            LexerError::UnexpectedEof(_) => "unexpected end of file",
            LexerError::UnterminatedString(_) => "unterminated string literal",
            LexerError::InvalidEscape(_, _) => "invalid escape sequence",
            LexerError::InvalidNumber(_, _) => "invalid integer",
        };
        Some(msg.to_string())
    }

    pub fn note(&self) -> Option<String> {
        let note = match self {
            LexerError::UnexpectedEof(_) => format!(
                "Expected one of the directives: {}",
                JasmTokenKind::list_directives()
            ),
            LexerError::UnexpectedChar(_, _, context) => return context.clone(),

            LexerError::UnterminatedString(_) => {
                "String literal is not terminated before the end of the line or file.".to_string()
            }
            LexerError::InvalidEscape(_, c) => {
                format!("The character '\\{}' is not a valid escape sequence.", c)
            }
            LexerError::UnknownDirective(_, _) => {
                format!("Valid directives are {}", JasmTokenKind::list_directives())
            }
            LexerError::InvalidNumber(_, value) => {
                if value.starts_with("0x") || value.starts_with("0X") {
                    "Hexadecimal numbers are not supported yet, but are planned for the future."
                        .to_string()
                } else {
                    "Integers must be between -2147483648 and 2147483647".to_string()
                }
            }
        };
        Some(note)
    }

    pub fn as_range(&self) -> Option<Range<usize>> {
        self.span().map(|s| s.as_range())
    }

    pub fn label(&self) -> Option<String> {
        let res = match self {
            LexerError::UnexpectedChar(_, c, _) => {
                format!("found '{}' here", c.escape_default())
            }
            LexerError::UnknownDirective(_, name) => {
                let mut closest = None;
                let mut min_dist = usize::MAX;
                for directive in JasmTokenKind::DIRECTIVES {
                    let d_str = directive.to_string();
                    let dist = crate::utils::levenshtein_distance(name, &d_str);
                    if dist < min_dist && dist <= 2 {
                        min_dist = dist;
                        closest = Some(d_str);
                    }
                }

                if let Some(suggestion) = closest {
                    format!("did you mean '{}'?", suggestion)
                } else {
                    "unknown directive".to_string()
                }
            }
            LexerError::UnexpectedEof(_) => "unexpected end of file".to_string(),
            LexerError::UnterminatedString(_) => {
                "this string literal is not terminated".to_string()
            }
            LexerError::InvalidEscape(_, c) => format!("invalid escape sequence '\\{}'", c),
            LexerError::InvalidNumber(_, value) => {
                if value.parse::<i128>().is_ok() {
                    format!(
                        "integer '{}' is too large for a 32-bit signed integer",
                        value
                    )
                } else if value.chars().any(|c| !c.is_digit(10) && c != '-') {
                    format!("'{}' contains invalid characters", value)
                } else {
                    format!("'{}' is not a valid integer", value)
                }
            }
        };
        Some(res)
    }

    fn span(&self) -> Option<&Span> {
        match self {
            LexerError::UnexpectedChar(span, _, _)
            | LexerError::UnknownDirective(span, _)
            | LexerError::UnexpectedEof(span)
            | LexerError::UnterminatedString(span)
            | LexerError::InvalidEscape(span, _)
            | LexerError::InvalidNumber(span, _) => Some(span),
        }
    }
}

impl From<LexerError> for JasmError {
    fn from(err: LexerError) -> Self {
        JasmError::Diagnostic(JasmDiagnostic::new(
            err.message().unwrap_or("lexing error".to_string()),
            err.as_range(),
            err.note(),
            err.label(),
        ))
    }
}
