use crate::ERROR_DOCS_BASE_URL;
use crate::diagnostic::{Diagnostic, DiagnosticLabel, DiagnosticTier};
use crate::token::{RnsToken, Span};

//TODO: same error code for all lexer, try to categorize later if needed

#[derive(Debug, Eq, PartialEq, Clone)]
pub(super) enum LexerError {
    UnknownDirective(Span, String),
    UnterminatedString(Span),
    UnknownTypeHint(Span, String),
    InvalidEscape(Span, char),
    InvalidInteger(Span, String),
}

impl LexerError {
    fn code(&self) -> Option<&'static str> {
        match self {
            LexerError::UnknownDirective(_, _) => Some("E-002"),
            LexerError::InvalidInteger(_, _) => Some("E-003"),
            LexerError::InvalidEscape(_, _) => Some("E-004"),
            LexerError::UnknownTypeHint(_, _) => Some("E-005"),
            _ => None,
        }
    }

    fn asm_msg(&self) -> String {
        match self {
            LexerError::UnknownDirective(_, _) => "unknown directive".to_string(),
            LexerError::UnterminatedString(_) => "unterminated string literal".to_string(),
            LexerError::InvalidEscape(_, _) => "invalid escape sequence".to_string(),
            LexerError::InvalidInteger(_, _) => "invalid integer".to_string(),
            LexerError::UnknownTypeHint(_, _) => "unknown type hint".to_string(),
        }
    }

    fn note(&self) -> Option<String> {
        match self {
            LexerError::UnterminatedString(_) => Some(
                "String literal is not terminated before the end of the line or file.".to_string(),
            ),
            LexerError::UnknownDirective(_, _)
            | LexerError::InvalidInteger(_, _)
            | LexerError::UnknownTypeHint(_, _)
            | LexerError::InvalidEscape(_, _) => self.code().map(|code| {
                format!(
                    "For more details see:\n{}{}",
                    ERROR_DOCS_BASE_URL,
                    code.to_ascii_lowercase()
                )
            }),
        }
    }

    fn labels(&self) -> Vec<DiagnosticLabel> {
        match self {
            LexerError::UnknownTypeHint(_, name) => {
                vec![DiagnosticLabel::at(
                    self.primary_location().as_range(),
                    format!("'{name}' is not a recognized type hint"),
                )]
            }
            LexerError::UnknownDirective(_, name) => {
                vec![DiagnosticLabel::at(
                    self.primary_location().as_range(),
                    format!("'{name}' is not a recognized directive"),
                )]
            }
            LexerError::UnterminatedString(_) => {
                vec![DiagnosticLabel::at(
                    self.primary_location().as_range(),
                    "this string literal is not terminated".to_string(),
                )]
            }
            LexerError::InvalidEscape(_, c) if *c == '\n' => vec![DiagnosticLabel::at(
                self.primary_location().as_range(),
                "newline characters cannot be escaped".to_string(),
            )],
            LexerError::InvalidEscape(_, c) => vec![DiagnosticLabel::at(
                self.primary_location().as_range(),
                format!("'\\{}' is not a valid escape sequence", c.escape_default()),
            )],
            LexerError::InvalidInteger(_, value) => {
                let msg = if value.parse::<i128>().is_ok() {
                    // TODO: suggest long when we support it
                    format!(
                        "integer '{}' is too large for a 32-bit signed integer",
                        value
                    )
                } else if value.chars().any(|c| !c.is_digit(10) && c != '-') {
                    format!("'{}' contains invalid characters", value)
                } else {
                    format!("'{}' is not a valid integer", value)
                };
                vec![DiagnosticLabel::at(self.primary_location().as_range(), msg)]
            }
        }
    }

    fn help(&self) -> Option<String> {
        match self {
            // TODO: add suggestions for type hints
            LexerError::UnknownDirective(_, name) => {
                let closest = RnsToken::closest_directive(name);
                closest.map(|closest| format!("Did you mean '{}'?", closest))
            }
            LexerError::InvalidInteger(_, value) => {
                if value.starts_with("0x") || value.starts_with("0X") {
                    Some("Hexadecimal numbers are not supported yet, but are planned for the future."
                        .to_string())
                } else {
                    Some("Integers must be between -2147483648 and 2147483647 to fit in a 32-bit signed integer.".to_string())
                }
            }
            LexerError::InvalidEscape(_, c) if *c == '\n' => Some(
                "Multiple lines string literals are not supported yet, but are planned for the future.".to_string(),
            ),
            _ => None,
        }
    }

    fn primary_location(&self) -> Span {
        match self {
            LexerError::UnknownDirective(span, _)
            | LexerError::UnterminatedString(span)
            | LexerError::InvalidEscape(span, _)
            | LexerError::UnknownTypeHint(span, _)
            | LexerError::InvalidInteger(span, _) => *span,
        }
    }

    fn lsp_msg(&self) -> String {
        match self {
            LexerError::UnknownDirective(_, name) => {
                format!("unknown directive '{name}'")
            }
            LexerError::UnterminatedString(_) => "unterminated string literal".to_string(),
            LexerError::UnknownTypeHint(_, name) => {
                format!("unknown type hint '{name}'")
            }
            LexerError::InvalidInteger(_, _) => "invalid 32-bit signed integer".to_string(),
            _ => self.asm_msg(),
        }
    }
}

impl From<LexerError> for Diagnostic {
    fn from(value: LexerError) -> Self {
        Diagnostic {
            asm_msg: value.asm_msg(),
            lsp_msg: value.lsp_msg(),
            code: value.code(),
            primary_location: value.primary_location(),
            note: value.note(),
            help: value.help(),
            tier: DiagnosticTier::SyntaxError,
            labels: value.labels(),
        }
    }
}
