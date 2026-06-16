use crate::diagnostic::{
    DiagnosticLabel, DiagnosticTier, ERR_CODE_UNDEFINED_LABEL, IntoDiagnostic, docs_note,
};
use crate::token::{Span, Spanned};
use std::borrow::Cow;

#[derive(Debug)]
pub(super) enum AssemblerError {
    UndefinedLabel { label: Spanned<String> },
}

impl IntoDiagnostic for AssemblerError {
    fn asm_msg(&self) -> Cow<'static, str> {
        match self {
            AssemblerError::UndefinedLabel { label } => {
                format!("undefined label '{}'", label.value).into()
            }
        }
    }

    fn lsp_msg(&self) -> Cow<'static, str> {
        self.asm_msg()
    }

    fn code(&self) -> &'static str {
        match self {
            AssemblerError::UndefinedLabel { .. } => ERR_CODE_UNDEFINED_LABEL,
        }
    }

    fn primary_location(&self) -> Span {
        match self {
            AssemblerError::UndefinedLabel { label } => label.span,
        }
    }

    fn note(&self) -> Option<Cow<'static, str>> {
        Some(docs_note(self.code()))
    }

    fn help(&self) -> Option<Cow<'static, str>> {
        match self {
            AssemblerError::UndefinedLabel { .. } => {
                Some("Labels must be defined within the same code directive.".into())
            }
        }
    }

    fn tier(&self) -> DiagnosticTier {
        DiagnosticTier::SyntaxError
    }

    fn labels(&self) -> Vec<DiagnosticLabel> {
        match self {
            AssemblerError::UndefinedLabel { label } => {
                vec![DiagnosticLabel::at(
                    label.span.as_range(),
                    format!(
                        "label '{}' is not defined in this code directive",
                        label.value
                    ),
                )]
            }
        }
    }
}
