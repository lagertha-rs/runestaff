use crate::diagnostic::{
    DiagnosticLabel, DiagnosticTier, ERR_CODE_PACKAGE_NON_CLASS_CONFLICT, ERR_CODE_UNDEFINED_LABEL,
    IntoDiagnostic, docs_note,
};
use crate::token::type_hint::TypeHint;
use crate::token::{Span, Spanned};
use std::borrow::Cow;

#[derive(Debug)]
pub(super) enum AssemblerError {
    UndefinedLabel {
        label: Spanned<String>,
    },
    PackageNonClassConflict {
        directive_name: &'static str,
        type_hint: TypeHint,
        package_span: Span,
    },
}

impl IntoDiagnostic for AssemblerError {
    fn asm_msg(&self) -> Cow<'static, str> {
        match self {
            AssemblerError::UndefinedLabel { label } => {
                format!("undefined label '{}'", label.value).into()
            }
            AssemblerError::PackageNonClassConflict {
                directive_name,
                type_hint,
                ..
            } => format!(
                ".package directive conflicts with {}: '{}' has type hint '{}' but .package requires @class",
                directive_name,
                type_hint_value(type_hint),
                type_hint_kind_name(type_hint),
            )
            .into(),
        }
    }

    fn lsp_msg(&self) -> Cow<'static, str> {
        self.asm_msg()
    }

    fn code(&self) -> &'static str {
        match self {
            AssemblerError::UndefinedLabel { .. } => ERR_CODE_UNDEFINED_LABEL,
            AssemblerError::PackageNonClassConflict { .. } => ERR_CODE_PACKAGE_NON_CLASS_CONFLICT,
        }
    }

    fn primary_location(&self) -> Span {
        match self {
            AssemblerError::UndefinedLabel { label } => label.span,
            AssemblerError::PackageNonClassConflict { type_hint, .. } => type_hint_span(type_hint),
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
            AssemblerError::PackageNonClassConflict { directive_name, .. } => Some(
                format!(
                    "Remove the .package directive or change {} to use @class type hint",
                    directive_name,
                )
                .into(),
            ),
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
            AssemblerError::PackageNonClassConflict {
                directive_name,
                type_hint,
                package_span,
            } => {
                let mut labels = vec![DiagnosticLabel::at(
                    type_hint_span(type_hint).as_range(),
                    format!(
                        "{} uses '{}' type hint, but .package can only be appended to @class entries",
                        directive_name,
                        type_hint_kind_name(type_hint),
                    ),
                )];
                labels.push(DiagnosticLabel::context(
                    package_span.as_range(),
                    ".package directive defined here",
                ));
                labels
            }
        }
    }
}

fn type_hint_kind_name(th: &TypeHint) -> &'static str {
    match th {
        TypeHint::Utf8(_, _) => "@utf8",
        TypeHint::String(_, _) => "@string",
        TypeHint::Integer(_, _) => "@int",
        TypeHint::Long(_, _) => "@long",
        TypeHint::Float(_, _) => "@float",
        TypeHint::Double(_, _) => "@double",
        TypeHint::CpIndex(_, _) => "@cp_idx",
        TypeHint::Class(_, _) => "@class",
        TypeHint::Methodref(_) => "@methodref",
        TypeHint::Fieldref(_) => "@fieldref",
        _ => "unknown",
    }
}

fn type_hint_value(th: &TypeHint) -> String {
    match th {
        TypeHint::Utf8(_, v) => v.value.clone(),
        TypeHint::String(_, v) => v.value.clone(),
        TypeHint::Class(_, v) => v.value.clone(),
        TypeHint::Integer(_, v) => v.value.to_string(),
        TypeHint::Long(_, v) => v.value.to_string(),
        TypeHint::Float(_, v) => v.value.to_string(),
        TypeHint::Double(_, v) => v.value.to_string(),
        TypeHint::CpIndex(_, v) => v.value.to_string(),
        TypeHint::Methodref(r) => {
            format!("{} {} {}", r.class.value, r.name.value, r.descriptor.value)
        }
        TypeHint::Fieldref(r) => {
            format!("{} {} {}", r.class.value, r.name.value, r.descriptor.value)
        }
        _ => String::new(),
    }
}

fn type_hint_span(th: &TypeHint) -> Span {
    match th {
        TypeHint::Utf8(s, v) => *s.as_ref().unwrap_or(&v.span),
        TypeHint::String(s, v) => *s.as_ref().unwrap_or(&v.span),
        TypeHint::Class(s, v) => *s.as_ref().unwrap_or(&v.span),
        TypeHint::Integer(s, v) => *s.as_ref().unwrap_or(&v.span),
        TypeHint::Long(s, v) => *s.as_ref().unwrap_or(&v.span),
        TypeHint::Float(s, v) => *s.as_ref().unwrap_or(&v.span),
        TypeHint::Double(s, v) => *s.as_ref().unwrap_or(&v.span),
        TypeHint::CpIndex(s, _v) => *s,
        TypeHint::Methodref(r) => r.hint_span.unwrap_or(r.class.span),
        TypeHint::Fieldref(r) => r.hint_span.unwrap_or(r.class.span),
        _ => Span::default(),
    }
}
