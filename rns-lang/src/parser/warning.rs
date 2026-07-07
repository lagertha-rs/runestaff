use crate::diagnostic::{DiagnosticLabel, DiagnosticTier, IntoDiagnostic};
use crate::parser::error::AccessFlagContext;
use crate::token::Span;
use crate::token::flag::RnsFlag;
use crate::token::type_hint::TypeHint;
use std::borrow::Cow;

#[derive(Debug)]
pub(super) enum ParserWarning {
    MissingSuperClass {
        class_name: Option<TypeHint>,
        class_dir_pos: Span,
        default: &'static str,
    },
    DuplicateAccessFlag {
        ctx: AccessFlagContext,
        flag: RnsFlag,
        spans: Vec<Span>,
    },
    ReservedLikeIdentifierTodoName,
    PackageContainsDot {
        package_name: String,
        package_span: Span,
    },
}

impl IntoDiagnostic for ParserWarning {
    fn code(&self) -> &'static str {
        match self {
            ParserWarning::MissingSuperClass { .. } => "W-001",
            ParserWarning::DuplicateAccessFlag { .. } => "TODO",
            ParserWarning::ReservedLikeIdentifierTodoName => "TODO",
            ParserWarning::PackageContainsDot { .. } => "W-002",
        }
    }

    fn asm_msg(&self) -> Cow<'static, str> {
        match self {
            ParserWarning::MissingSuperClass { .. } => "missing super directive".into(),
            ParserWarning::DuplicateAccessFlag { ctx, flag, .. } => format!(
                "duplicate access flag '{}' in {} definition",
                flag.name(),
                ctx
            )
            .into(),
            ParserWarning::ReservedLikeIdentifierTodoName => {
                "TODO: reserved-like identifier used as name".into()
            }
            ParserWarning::PackageContainsDot { package_name, .. } => {
                format!("package name '{}' contains '.' separator", package_name).into()
            }
        }
    }

    fn primary_location(&self) -> Span {
        match self {
            ParserWarning::MissingSuperClass {
                class_dir_pos: class_directive_pos,
                ..
            } => *class_directive_pos,
            ParserWarning::DuplicateAccessFlag { spans, .. } => {
                spans.get(1).copied().unwrap_or_default()
            }
            ParserWarning::ReservedLikeIdentifierTodoName => Span::default(),
            ParserWarning::PackageContainsDot { package_span, .. } => *package_span,
        }
    }

    fn note(&self) -> Option<Cow<'static, str>> {
        match self {
            ParserWarning::MissingSuperClass { default, .. } => Some(
                format!(
                    "The .super directive is required to specify the superclass. \
                     Defaulting to '{}'.",
                    default
                )
                .into(),
            ),
            ParserWarning::DuplicateAccessFlag { flag, .. } => Some(
                format!(
                    "The `{}` flag was already specified. You only need to declare it once.",
                    flag.name()
                )
                .into(),
            ),
            ParserWarning::ReservedLikeIdentifierTodoName => {
                Some("TODO: reserved-like identifier used as name".into())
            }
            ParserWarning::PackageContainsDot { .. } => Some(
                "In bytecode, package separators are represented as '/'. \
                 The '.' character is used in Java source syntax but will be kept as-is in the bytecode."
                    .into(),
            ),
        }
    }

    fn labels(&self) -> Vec<DiagnosticLabel> {
        match self {
            ParserWarning::MissingSuperClass {
                class_dir_pos: class_directive_pos,
                class_name,
                ..
            } => {
                let class_desc = match class_name {
                    Some(name) => name.value(),
                    None => "<unknown>".to_string(),
                };
                vec![DiagnosticLabel::at(
                    class_directive_pos.as_range(),
                    format!("class '{}' is missing a '.super' directive", class_desc),
                )]
            }
            ParserWarning::DuplicateAccessFlag { flag: _, spans, .. } => {
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
            ParserWarning::PackageContainsDot { package_name, .. } => {
                vec![DiagnosticLabel::at(
                    self.primary_location().as_range(),
                    format!("package '{}' uses '.' as separator", package_name),
                )]
            }
        }
    }

    fn help(&self) -> Option<Cow<'static, str>> {
        match self {
            ParserWarning::PackageContainsDot { package_name, .. } => {
                let suggested = package_name.replace('.', "/");
                Some(
                    format!(
                        "If you intended Java-style package syntax, use '/' instead: '{}'",
                        suggested
                    )
                    .into(),
                )
            }
            _ => None,
        }
    }

    fn tier(&self) -> DiagnosticTier {
        DiagnosticTier::AssemblerWarn
    }
}
