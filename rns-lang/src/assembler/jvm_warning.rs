use crate::diagnostic::{Diagnostic, DiagnosticLabel, DiagnosticTier};
use crate::token::Span;
use std::ops::Range;
use strum::EnumProperty;

#[derive(Debug, EnumProperty)]
pub(super) enum JvmWarning {
    #[strum(props(code = "JVMS001"))]
    InterfaceFlagWithMissingAbstract { interface_span: Span },
}

impl JvmWarning {
    fn message(&self) -> String {
        match self {
            JvmWarning::InterfaceFlagWithMissingAbstract { .. } => {
                "interface must be declared with the 'abstract' access flag".to_string()
            }
        }
    }

    fn primary_location(&self) -> Range<usize> {
        match self {
            JvmWarning::InterfaceFlagWithMissingAbstract { interface_span } => {
                interface_span.as_range()
            }
        }
    }

    fn note(&self) -> Option<String> {
        match self {
            JvmWarning::InterfaceFlagWithMissingAbstract { .. } => Some(
                "In the JVM specification, interfaces are implicitly abstract, but they must still be declared with the 'abstract' access flag to be valid.".to_string(),
            ),
        }
    }

    fn help(&self) -> Option<String> {
        match self {
            JvmWarning::InterfaceFlagWithMissingAbstract { .. } => Some(
                "Add the 'abstract' access flag to the class declaration to fix this warning."
                    .to_string(),
            ),
        }
    }

    fn labels(&self) -> Vec<DiagnosticLabel> {
        match self {
            JvmWarning::InterfaceFlagWithMissingAbstract { interface_span } => {
                vec![DiagnosticLabel::at(
                    interface_span.as_range(),
                    "the 'interface' access flag is declared here".to_string(),
                )]
            }
        }
    }
}

impl From<JvmWarning> for Diagnostic {
    fn from(value: JvmWarning) -> Self {
        Diagnostic {
            message: value.message(),
            code: value.get_str("code").unwrap_or("JVMS000"),
            primary_location: value.primary_location(),
            note: value.note(),
            help: value.help(),
            tier: DiagnosticTier::JvmSpecWarn,
            labels: value.labels(),
        }
    }
}
