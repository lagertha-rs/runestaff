use crate::ERROR_DOCS_BASE_URL;
use crate::diagnostic::{Diagnostic, DiagnosticLabel, DiagnosticTier};
use crate::suggestion;
use crate::token::Span;

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
    fn code(&self) -> &'static str {
        match self {
            LexerError::UnterminatedString(_) => "E-001",
            LexerError::UnknownDirective(_, _) => "E-002",
            LexerError::InvalidInteger(_, _) => "E-003",
            LexerError::InvalidEscape(_, _) => "E-004",
            LexerError::UnknownTypeHint(_, _) => "E-005",
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

    fn note(&self) -> String {
        format!(
            "For more details see:\n{}{}",
            ERROR_DOCS_BASE_URL,
            self.code().to_ascii_lowercase()
        )
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
            LexerError::InvalidEscape(_, c) if *c == '\r' => vec![DiagnosticLabel::at(
                self.primary_location().as_range(),
                "carriage return characters cannot be escaped".to_string(),
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
            LexerError::UnknownDirective(_, name) => suggestion::closest_directive(name)
                .map(|closest| format!("Did you mean '{}'?", closest)),
            LexerError::UnknownTypeHint(_, name) => {
                let bare = name.strip_prefix('@').unwrap_or(name);
                suggestion::closest_type_hint(bare)
                    .map(|closest| format!("Did you mean '@{}'?", closest))
            }
            LexerError::InvalidInteger(_, value) => {
                if value.starts_with("0x") || value.starts_with("0X") {
                    Some("Hexadecimal numbers are not supported yet, but are planned for the future."
                        .to_string())
                } else if value.chars().any(|c| !c.is_ascii_digit() && c != '-') {
                    Some(
                        "Integers can only contain digits and an optional leading minus sign."
                            .to_string(),
                    )
                } else {
                    Some("Integers must be between -2147483648 and 2147483647 to fit in a 32-bit signed integer.".to_string())
                }
            }
            LexerError::InvalidEscape(_, c) if *c == '\n' || *c == '\r' => Some(
                "Multiline string literals are not supported yet, but are planned for the future."
                    .to_string(),
            ),
            LexerError::InvalidEscape(_, _) => None,
            LexerError::UnterminatedString(_) => {
                Some("Close the string with a '\"' on the same line.".to_string())
            }
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
            code: Some(value.code()),
            primary_location: value.primary_location(),
            note: Some(value.note()),
            help: value.help(),
            tier: DiagnosticTier::SyntaxError,
            labels: value.labels(),
        }
    }
}
