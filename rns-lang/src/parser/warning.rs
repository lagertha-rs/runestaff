use crate::diagnostic::{Diagnostic, DiagnosticLabel, DiagnosticTier};
use crate::token::type_hint::TypeHint;
use crate::token::{RnsFlag, Span};

#[derive(Debug)]
pub(super) enum ParserWarning {
    MissingSuperClass {
        class_name: Option<TypeHint>,
        class_dir_pos: Span,
        default: &'static str,
    },
    ClassDuplicateFlag {
        flag: RnsFlag,
        spans: Vec<Span>,
    },
    ReservedLikeIdentifierTodoName,
}

impl ParserWarning {
    fn code(&self) -> &'static str {
        match self {
            ParserWarning::MissingSuperClass { .. } => "W-001",
            ParserWarning::ClassDuplicateFlag { .. } => "TODO",
            ParserWarning::ReservedLikeIdentifierTodoName => "TODO",
        }
    }
    fn asm_msg(&self) -> String {
        match self {
            ParserWarning::MissingSuperClass { .. } => "missing super directive".to_string(),
            ParserWarning::ClassDuplicateFlag { flag, .. } => {
                format!(
                    "duplicate access flag '{}' in class definition",
                    flag.name()
                )
            }
            ParserWarning::ReservedLikeIdentifierTodoName => {
                "TODO: reserved-like identifier used as name".to_string()
            }
        }
    }

    fn primary_location(&self) -> Span {
        match self {
            ParserWarning::MissingSuperClass {
                class_dir_pos: class_directive_pos,
                ..
            } => *class_directive_pos,
            ParserWarning::ClassDuplicateFlag { spans, .. } => {
                spans.get(1).copied().unwrap_or_default()
            }
            ParserWarning::ReservedLikeIdentifierTodoName => Span::default(),
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
                flag.name()
            )),
            ParserWarning::ReservedLikeIdentifierTodoName => {
                Some("TODO: reserved-like identifier used as name".to_string())
            }
        }
    }

    fn labels(&self) -> Vec<DiagnosticLabel> {
        match self {
            ParserWarning::MissingSuperClass {
                class_dir_pos: class_directive_pos,
                class_name,
                ..
            } => vec![DiagnosticLabel::at(
                class_directive_pos.as_range(),
                format!("class '{:?}' is missing a '.super' directive", class_name),
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
            ParserWarning::ReservedLikeIdentifierTodoName => vec![],
        }
    }

    fn lsp_msg(&self) -> String {
        //TODO: stub
        self.asm_msg()
    }
}

impl From<ParserWarning> for Diagnostic {
    fn from(value: ParserWarning) -> Self {
        Diagnostic {
            asm_msg: value.asm_msg(),
            lsp_msg: value.lsp_msg(),
            code: Some(value.code()),
            primary_location: value.primary_location(),
            note: value.note(),
            help: None,
            tier: DiagnosticTier::AssemblerWarn,
            labels: value.labels(),
        }
    }
}
