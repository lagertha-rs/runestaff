use crate::diagnostic::{Diagnostic, DiagnosticLabel, DiagnosticTier, Severity};
use crate::token::{JasmAccessFlag, Span};
use std::ops::Range;

#[derive(Debug)]
pub(super) enum ParserWarning {
    MissingSuperClass {
        class_name: String,
        class_directive_pos: Span,
        default: &'static str,
    },
    ClassDuplicateFlag {
        flag: JasmAccessFlag,
        spans: Vec<Span>,
    },
}

impl Diagnostic for ParserWarning {
    fn message(&self) -> String {
        match self {
            ParserWarning::MissingSuperClass { .. } => "missing super directive".to_string(),
            ParserWarning::ClassDuplicateFlag { flag, .. } => {
                format!("duplicate access flag '{}'", flag)
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

    fn tier(&self) -> DiagnosticTier {
        DiagnosticTier::Assembler // TODO: stub
    }

    fn note(&self) -> Option<String> {
        match self {
            ParserWarning::MissingSuperClass { default, .. } => Some(format!(
                "The .super directive is required to specify the superclass. \
                 Defaulting to '{}'.",
                default
            )),
            ParserWarning::ClassDuplicateFlag { .. } => Some(
                "This flag was already specified. You only need to declare it once.".to_string(),
            ),
        }
    }

    fn severity(&self) -> Severity {
        Severity::Warning
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
