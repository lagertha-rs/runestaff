use crate::diagnostic::{Diagnostic, DiagnosticLabel, DiagnosticTier, Severity};
use crate::token::Span;
use std::ops::Range;

#[derive(Debug)]
pub(super) enum JvmWarning {
    MissingSuperClass {
        class_name: String,
        class_directive_pos: Span,
        default: &'static str,
    },
}

impl Diagnostic for JvmWarning {
    fn message(&self) -> String {
        match self {
            JvmWarning::MissingSuperClass { .. } => "missing super directive".to_string(),
        }
    }

    fn primary_location(&self) -> Range<usize> {
        match self {
            JvmWarning::MissingSuperClass {
                class_directive_pos,
                ..
            } => class_directive_pos.as_range(),
        }
    }

    fn tier(&self) -> DiagnosticTier {
        DiagnosticTier::JvmSpec // TODO: stub
    }

    fn note(&self) -> Option<String> {
        match self {
            JvmWarning::MissingSuperClass { default, .. } => Some(format!(
                "The .super directive is required to specify the superclass. \
                 Defaulting to '{}'.",
                default
            )),
        }
    }

    fn severity(&self) -> Severity {
        Severity::Warning
    }

    fn labels(&self) -> Vec<DiagnosticLabel> {
        match self {
            JvmWarning::MissingSuperClass {
                class_directive_pos,
                class_name,
                ..
            } => vec![DiagnosticLabel::at(
                class_directive_pos.as_range(),
                format!("class '{}' is missing a '.super' directive", class_name),
            )],
        }
    }
}
