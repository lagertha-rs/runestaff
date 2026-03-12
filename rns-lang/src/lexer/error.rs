use crate::diagnostic::{Diagnostic, DiagnosticLabel, DiagnosticTier};
use crate::suggestion;
use crate::token::Span;
use crate::ERROR_DOCS_BASE_URL;

//TODO: same error code for all lexer, try to categorize later if needed

#[derive(Debug, Eq, PartialEq, Clone)]
pub(super) enum LexerError {
    UnknownDirective(Span, String),
    UnterminatedQuotedIdentifier(Span),
    UnknownTypeHint(Span, String),
    InvalidEscape(Span, char),
}

impl LexerError {
    fn code(&self) -> &'static str {
        match self {
            LexerError::UnterminatedQuotedIdentifier(_) => "E-001",
            LexerError::UnknownDirective(_, _) => "E-002",
            LexerError::InvalidEscape(_, _) => "E-004",
            LexerError::UnknownTypeHint(_, _) => "E-005",
        }
    }

    fn asm_msg(&self) -> String {
        match self {
            LexerError::UnknownDirective(_, _) => "unknown directive".to_string(),
            LexerError::UnterminatedQuotedIdentifier(_) => {
                "unterminated quoted identifier".to_string()
            }
            LexerError::InvalidEscape(_, _) => "invalid escape sequence".to_string(),
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
            LexerError::UnterminatedQuotedIdentifier(_) => {
                vec![DiagnosticLabel::at(
                    self.primary_location().as_range(),
                    "this identifier is not terminated".to_string(),
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
            LexerError::InvalidEscape(_, c) if *c == '\n' || *c == '\r' => Some(
                "Multiline identifiers are not supported yet, but are planned for the future."
                    .to_string(),
            ),
            LexerError::InvalidEscape(_, _) => None,
            LexerError::UnterminatedQuotedIdentifier(_) => {
                Some("Close the identifier with a '\"' on the same line.".to_string())
            }
        }
    }

    fn primary_location(&self) -> Span {
        match self {
            LexerError::UnknownDirective(span, _)
            | LexerError::UnterminatedQuotedIdentifier(span)
            | LexerError::InvalidEscape(span, _)
            | LexerError::UnknownTypeHint(span, _) => *span,
        }
    }

    fn lsp_msg(&self) -> String {
        match self {
            LexerError::UnknownDirective(_, name) => {
                format!("unknown directive '{name}'")
            }
            LexerError::UnterminatedQuotedIdentifier(_) => {
                "unterminated quoted identifier".to_string()
            }
            LexerError::UnknownTypeHint(_, name) => {
                format!("unknown type hint '{name}'")
            }
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
