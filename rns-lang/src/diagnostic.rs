use crate::token::Span;
use ariadne::{Color, Fmt, Label, Report, ReportKind, Source};
use std::fmt::Debug;
use std::ops::Range;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiagnosticLabel {
    pub span: Range<usize>,
    pub message: String,
    color: Option<Color>,
}

impl DiagnosticLabel {
    /// With default color (same as severity)
    pub fn at(span: Range<usize>, message: impl Into<String>) -> Self {
        Self {
            span,
            message: message.into(),
            color: None,
        }
    }

    /// With other color, for example to highlight context in a different color than the main error
    pub fn context(span: Range<usize>, message: impl Into<String>) -> Self {
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
    pub asm_msg: String,
    pub lsp_msg: String,
    pub code: Option<&'static str>,
    pub primary_location: Span,
    pub note: Option<String>,
    pub help: Option<String>,
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
