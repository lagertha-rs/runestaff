use crate::ERROR_DOCS_BASE_URL;
use crate::token::Span;
use ariadne::{Color, Fmt, Label, Report, ReportKind, Source};
use std::borrow::Cow;
use std::fmt::Debug;
use std::ops::Range;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiagnosticLabel {
    pub span: Range<usize>,
    pub message: Cow<'static, str>,
    color: Option<Color>,
}

impl DiagnosticLabel {
    /// With default color (same as severity)
    pub fn at(span: Range<usize>, message: impl Into<Cow<'static, str>>) -> Self {
        Self {
            span,
            message: message.into(),
            color: None,
        }
    }

    /// With other color, for example to highlight context in a different color than the main error
    pub fn context(span: Range<usize>, message: impl Into<Cow<'static, str>>) -> Self {
        Self {
            span,
            message: message.into(),
            color: Some(Color::BrightCyan),
        }
    }

    fn get_color(&self, tier: DiagnosticTier) -> Color {
        self.color.unwrap_or_else(|| tier.into())
    }
}

#[derive(Debug)]
pub struct Diagnostic {
    pub asm_msg: Cow<'static, str>,
    pub lsp_msg: Cow<'static, str>,
    pub code: Option<&'static str>,
    pub primary_location: Span,
    pub note: Option<Cow<'static, str>>,
    pub help: Option<Cow<'static, str>>,
    pub tier: DiagnosticTier,
    pub labels: Vec<DiagnosticLabel>,
}

impl Diagnostic {
    pub fn print(self, filename: &str, source_code: &str) {
        let range = self.primary_location.as_range();
        let filename_owned = filename.to_string();
        let mut report = Report::build(self.tier.into(), (filename_owned.clone(), range.clone()))
            .with_message(self.asm_msg);

        if let Some(code) = self.code {
            report = report.with_code(code);
        }

        if let Some(note) = self.note {
            report = report.with_note(note);
        }

        if let Some(help) = self.help {
            report = report.with_help(help);
        }

        for label in self.labels {
            let color = label.get_color(self.tier);
            let ariadne_label =
                Label::new((filename_owned.clone(), label.span.clone())).with_color(color);

            let ariadne_label = if label.message.is_empty() {
                ariadne_label
            } else {
                ariadne_label.with_message(label.message.fg(color))
            };

            report = report.with_label(ariadne_label);
        }

        report
            .finish()
            .eprint((filename_owned, Source::from(source_code)))
            .unwrap();
    }
}

pub(crate) trait IntoDiagnostic {
    fn asm_msg(&self) -> Cow<'static, str>;

    fn lsp_msg(&self) -> Cow<'static, str> {
        self.asm_msg()
    }

    fn code(&self) -> &'static str;
    fn primary_location(&self) -> Span;
    fn note(&self) -> Option<Cow<'static, str>>;
    fn help(&self) -> Option<Cow<'static, str>>;
    fn tier(&self) -> DiagnosticTier;
    fn labels(&self) -> Vec<DiagnosticLabel>;
}

impl<T: IntoDiagnostic> From<T> for Diagnostic {
    fn from(value: T) -> Self {
        Diagnostic {
            asm_msg: value.asm_msg(),
            lsp_msg: value.lsp_msg(),
            code: Some(value.code()),
            primary_location: value.primary_location(),
            note: value.note(),
            help: value.help(),
            tier: value.tier(),
            labels: value.labels(),
        }
    }
}

pub(crate) fn docs_note(code: &str) -> Cow<'static, str> {
    format!("For more details see:\n{}{}", ERROR_DOCS_BASE_URL, code).into()
}

pub(crate) fn jvms_docs_note(code: &str) -> Cow<'static, str> {
    format!(
        "If this violation isn't intentional, see details at:\n{}{}",
        ERROR_DOCS_BASE_URL, code
    )
    .into()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticTier {
    SyntaxError,   // Can't parse - always error
    AssemblerWarn, // Assembler logic issues
    JvmSpecWarn,   // JVM spec violations
}

impl From<DiagnosticTier> for ReportKind<'_> {
    fn from(tier: DiagnosticTier) -> Self {
        match tier {
            DiagnosticTier::SyntaxError => ReportKind::Error,
            DiagnosticTier::AssemblerWarn | DiagnosticTier::JvmSpecWarn => ReportKind::Warning,
        }
    }
}

impl From<DiagnosticTier> for Color {
    fn from(tier: DiagnosticTier) -> Self {
        match tier {
            DiagnosticTier::SyntaxError => Color::Red,
            DiagnosticTier::AssemblerWarn | DiagnosticTier::JvmSpecWarn => Color::Yellow,
        }
    }
}

impl From<Diagnostic> for Vec<Diagnostic> {
    fn from(value: Diagnostic) -> Self {
        vec![value]
    }
}
