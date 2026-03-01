use crate::diagnostic::{Diagnostic, DiagnosticLabel, DiagnosticTier, ERROR_DOCS_BASE_URL};
use crate::token::{RnsFlag, Span};
use std::ops::Range;

#[derive(Debug)]
pub(super) enum JvmWarning {
    InterfaceFlagWithMissingAbstract {
        interface_span: Span,
    },
    InterfaceMutuallyExclusive {
        interface_span: Span,
        exclusive_flags: Vec<(RnsFlag, Span)>,
    },
}

impl JvmWarning {
    fn code(&self) -> &'static str {
        match self {
            JvmWarning::InterfaceFlagWithMissingAbstract { .. }
            | JvmWarning::InterfaceMutuallyExclusive { .. } => "JVMS001",
        }
    }
    fn message(&self) -> String {
        match self {
            JvmWarning::InterfaceFlagWithMissingAbstract { .. } => {
                "interface must also be declared as 'abstract'".to_string()
            }
            JvmWarning::InterfaceMutuallyExclusive { .. } => {
                "interface cannot be declared with mutually exclusive flags".to_string()
            }
        }
    }

    fn primary_location(&self) -> Range<usize> {
        match self {
            JvmWarning::InterfaceFlagWithMissingAbstract { interface_span } => {
                interface_span.as_range()
            }
            JvmWarning::InterfaceMutuallyExclusive { interface_span, .. } => {
                interface_span.as_range()
            }
        }
    }

    fn note(&self) -> String {
        format!(
            "If this violation isn't intentional, see details at:\n{}{}",
            ERROR_DOCS_BASE_URL,
            self.code().to_ascii_lowercase()
        )
    }

    fn help(&self) -> Option<String> {
        match self {
            JvmWarning::InterfaceFlagWithMissingAbstract { .. } => {
                Some("Add the 'abstract' access flag to the class declaration.".to_string())
            }
            JvmWarning::InterfaceMutuallyExclusive {
                exclusive_flags, ..
            } => {
                let flags_list = exclusive_flags
                    .iter()
                    .map(|(flag, _)| format!("'{}'", flag.jvm_spec_name()))
                    .collect::<Vec<_>>()
                    .join(", ");
                Some(format!("Consider removing: {}.", flags_list))
            }
        }
    }

    fn labels(&self) -> Vec<DiagnosticLabel> {
        match self {
            JvmWarning::InterfaceFlagWithMissingAbstract { interface_span } => {
                vec![DiagnosticLabel::at(
                    interface_span.as_range(),
                    format!(
                        "'{}' requires '{}'",
                        RnsFlag::Interface.jvm_spec_name(),
                        RnsFlag::Abstract.jvm_spec_name()
                    ),
                )]
            }
            JvmWarning::InterfaceMutuallyExclusive {
                interface_span,
                exclusive_flags,
            } => {
                let mut labels = vec![DiagnosticLabel::context(
                    interface_span.as_range(),
                    format!("'{}' is declared here", RnsFlag::Interface.jvm_spec_name(),),
                )];
                for (flag, span) in exclusive_flags {
                    labels.push(DiagnosticLabel::at(
                        span.as_range(),
                        format!("'{}' is exclusive", flag.jvm_spec_name()),
                    ));
                }
                labels
            }
        }
    }
}

impl From<JvmWarning> for Diagnostic {
    fn from(value: JvmWarning) -> Self {
        Diagnostic {
            message: value.message(),
            code: value.code(),
            primary_location: value.primary_location(),
            note: Some(value.note()),
            help: value.help(),
            tier: DiagnosticTier::JvmSpecWarn,
            labels: value.labels(),
        }
    }
}
