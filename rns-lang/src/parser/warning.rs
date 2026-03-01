use crate::diagnostic::{Diagnostic, DiagnosticLabel, DiagnosticTier};
use crate::token::{RnsFlag, Span};
use std::ops::Range;

#[derive(Debug)]
pub(super) enum ParserWarning {
    MissingSuperClass {
        class_name: String,
        class_directive_pos: Span,
        default: &'static str,
    },
    ClassDuplicateFlag {
        flag: RnsFlag,
        spans: Vec<Span>,
    },
}

impl ParserWarning {
    fn code(&self) -> &'static str {
        match self {
            ParserWarning::MissingSuperClass { .. } => "W001",
            ParserWarning::ClassDuplicateFlag { .. } => "W002",
        }
    }
    fn message(&self) -> String {
        match self {
            ParserWarning::MissingSuperClass { .. } => "missing super directive".to_string(),
            ParserWarning::ClassDuplicateFlag { flag, .. } => {
                format!("duplicate access flag '{}' in class declaration", flag)
            }
        }
    }

    fn primary_location(&self) -> Range<usize> {
        match self {
            ParserWarning::MissingSuperClass {
                class_directive_pos,
                ..
            } => class_directive_pos.as_range(),
            ParserWarning::ClassDuplicateFlag { spans, .. } => {
                spans.get(1).copied().unwrap_or_default().as_range()
            }
        }
    }

    fn note(&self) -> Option<String> {
        match self {
            ParserWarning::MissingSuperClass { default, .. } => Some(format!(
                "The .super directive is required to specify the superclass. \
                 Defaulting to '{}'.",
                default
            )),
            ParserWarning::ClassDuplicateFlag { flag, .. } => Some(format!(
                "The `{}` flag was already specified. You only need to declare it once.",
                flag
            )),
        }
    }

    fn labels(&self) -> Vec<DiagnosticLabel> {
        match self {
            ParserWarning::MissingSuperClass {
                class_directive_pos,
                class_name,
                ..
            } => vec![DiagnosticLabel::at(
                class_directive_pos.as_range(),
                format!("class '{}' is missing a '.super' directive", class_name),
            )],
            ParserWarning::ClassDuplicateFlag { flag, spans } => {
                let mut labels = Vec::with_capacity(spans.len());
                labels.push(DiagnosticLabel::context(
                    spans[0].as_range(),
                    "first defined here",
                ));
                for span in spans.iter().skip(1) {
                    labels.push(DiagnosticLabel::at(
                        span.as_range(),
                        "duplicate flag ignored here",
                    ))
                }
                labels
            }
        }
    }
}

impl From<ParserWarning> for Diagnostic {
    fn from(value: ParserWarning) -> Self {
        Diagnostic {
            message: value.message(),
            code: value.code(),
            primary_location: value.primary_location(),
            note: value.note(),
            help: None,
            tier: DiagnosticTier::AssemblerWarn,
            labels: value.labels(),
        }
    }
}
