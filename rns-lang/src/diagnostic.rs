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

    fn get_color(&self, severity: Severity) -> Color {
        self.color.unwrap_or_else(|| severity.color())
    }
}

pub trait Diagnostic: Debug {
    fn message(&self) -> String;
    fn primary_location(&self) -> Range<usize>;
    fn note(&self) -> Option<String>;
    fn severity(&self) -> Severity;
    fn labels(&self) -> Vec<DiagnosticLabel>;

    fn print(&self, filename: &str, source_code: &str) {
        let range = self.primary_location();
        let filename_owned = filename.to_string();
        let mut report = Report::build(
            self.severity().into(),
            (filename_owned.clone(), range.clone()),
        )
        .with_message(self.message());

        if let Some(note) = self.note() {
            report = report.with_note(note);
        }

        for label in self.labels() {
            let color = label.get_color(self.severity());
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
pub enum Severity {
    Error,
    Warning,
}

impl Severity {
    pub fn color(&self) -> Color {
        match self {
            Severity::Error => Color::Red,
            Severity::Warning => Color::Yellow,
        }
    }
}

impl From<Severity> for ReportKind<'_> {
    fn from(severity: Severity) -> Self {
        match severity {
            Severity::Error => ReportKind::Error,
            Severity::Warning => ReportKind::Warning,
        }
    }
}

pub enum JasmError {
    Diagnostic(Box<dyn Diagnostic>),
    Internal(String),
}

impl Debug for JasmError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            JasmError::Diagnostic(_) => write!(f, "JasmError::Diagnostic(<diagnostic>)"),
            JasmError::Internal(msg) => write!(f, "JasmError::Internal({})", msg),
        }
    }
}

impl JasmError {
    fn format_internal_error(message: &str) -> String {
        [
            "",
            "!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!",
            "!            INTERNAL ASSEMBLER ERROR             !",
            "!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!",
            "",
            "This is a bug in the jasm assembler.",
            "Please report it at: https://github.com/Obito-git/lagertha-vm/issues",
            "",
            &format!("Details: {message}"),
        ]
        .join("\n")
    }

    pub fn print(&self, filename: &str, source_code: &str) {
        match self {
            JasmError::Diagnostic(diag) => {
                diag.print(filename, source_code);
            }
            JasmError::Internal(msg) => {
                eprintln!("{}", Self::format_internal_error(msg));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_internal_error() {
        let output = JasmError::format_internal_error("unexpected state in parser");

        insta::assert_snapshot!(output);
    }
}
