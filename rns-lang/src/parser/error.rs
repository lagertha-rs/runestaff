use crate::ERROR_DOCS_BASE_URL;
use crate::diagnostic::{Diagnostic, DiagnosticLabel, DiagnosticTier};
use crate::token::{RnsToken, Span};

#[derive(Debug, Eq, PartialEq, Clone)]
pub(super) enum ParserError {
    EmptyFile(Span),
    UnexpectedTokenInClassBody(RnsToken),
    UnexpectedTokenBeforeClassDefinition(RnsToken),
}

impl ParserError {
    // TODO: put all codes as consts somewhere, and make sure they are unique across the whole codebase
    fn code(&self) -> &'static str {
        match self {
            ParserError::EmptyFile(_) => "E-006",
            ParserError::UnexpectedTokenInClassBody(_) => "E-007",
            ParserError::UnexpectedTokenBeforeClassDefinition(_) => "E-008",
        }
    }

    fn asm_msg(&self) -> String {
        match self {
            ParserError::EmptyFile(_) => "file contains no class definition".to_string(),
            ParserError::UnexpectedTokenInClassBody(token) => {
                format!("unexpected token in class body: '{token:?}'")
            }
            ParserError::UnexpectedTokenBeforeClassDefinition(unexpected) => {
                format!(
                    "unexpected {} before class definition",
                    unexpected.token_type()
                )
            }
        }
    }

    fn labels(&self) -> Vec<DiagnosticLabel> {
        match self {
            ParserError::EmptyFile(_) => {
                vec![DiagnosticLabel::at(0..0, "expected a '.class' directive")]
            }
            ParserError::UnexpectedTokenInClassBody(token) => {
                vec![DiagnosticLabel::at(
                    token.span().as_range(),
                    "unexpected token",
                )]
            }
            ParserError::UnexpectedTokenBeforeClassDefinition(unexpected) => {
                let valid_ctx = unexpected.can_appear_in();
                let msg = format!(
                    "valid context{} for {} {} {}: {}.",
                    if valid_ctx.len() == 1 { "" } else { "s" },
                    unexpected,
                    unexpected.token_type(),
                    if valid_ctx.len() == 1 { "is" } else { "are" },
                    valid_ctx
                        .iter()
                        .map(|ctx| format!("'{}'", ctx))
                        .collect::<Vec<_>>()
                        .join(", ")
                );
                vec![DiagnosticLabel::at(unexpected.span().as_range(), msg)]
            }
        }
    }

    fn note(&self) -> String {
        format!(
            "For more details see:\n{}{}",
            ERROR_DOCS_BASE_URL,
            self.code().to_ascii_lowercase()
        )
    }

    fn help(&self) -> Option<String> {
        match self {
            ParserError::EmptyFile(_) => {
                Some("Make sure the file contains a valid class definition.".to_string())
            }
            ParserError::UnexpectedTokenInClassBody(_) => Some(
                "Check the syntax of the class body and ensure all tokens are valid.".to_string(),
            ),
            ParserError::UnexpectedTokenBeforeClassDefinition(_) => {
                Some("Make sure the file starts with a '.class' directive.".to_string())
            }
        }
    }

    fn primary_location(&self) -> Span {
        match self {
            ParserError::EmptyFile(span) => *span,
            ParserError::UnexpectedTokenInClassBody(token)
            | ParserError::UnexpectedTokenBeforeClassDefinition(token) => token.span(),
        }
    }

    fn lsp_message(&self) -> String {
        // TODO: stub
        self.asm_msg()
    }
}

impl From<ParserError> for Diagnostic {
    fn from(value: ParserError) -> Self {
        Diagnostic {
            asm_msg: value.asm_msg(),
            lsp_msg: value.lsp_message(),
            code: Some(value.code()),
            primary_location: value.primary_location(),
            note: Some(value.note()),
            help: value.help(),
            tier: DiagnosticTier::SyntaxError,
            labels: value.labels(),
        }
    }
}

impl From<ParserError> for Vec<Diagnostic> {
    fn from(value: ParserError) -> Self {
        vec![Diagnostic::from(value)]
    }
}
