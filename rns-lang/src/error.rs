use crate::lexer::LexerError;
use crate::parser::ParserError;
use ariadne::{Color, Label, Report, ReportKind, Source};
use std::ops::Range;

#[derive(Debug)]
pub enum JasmError {
    Diagnostic(JasmDiagnostic),
    Internal(String),
}

impl JasmError {
    fn print_diagnostic_error(filename: &str, source_code: &str, err: JasmDiagnostic) {
        let range = err.range().cloned().unwrap_or(0..0);
        let mut report =
            Report::build(ReportKind::Error, (filename, range.clone())).with_message(err.message());

        if let Some(note) = err.note() {
            report = report.with_note(note);
        }

        if let Some(label) = err.label() {
            report = report.with_label(
                Label::new((filename, range))
                    .with_message(label)
                    .with_color(Color::Red),
            );
        }

        report
            .finish()
            .eprint((filename, Source::from(source_code)))
            .unwrap();
    }

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
                Self::print_diagnostic_error(filename, source_code, diag.clone())
            }
            JasmError::Internal(msg) => {
                eprintln!("{}", Self::format_internal_error(msg));
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JasmDiagnostic {
    message: String,
    range: Option<Range<usize>>,
    note: Option<String>,
    label: Option<String>,
}

impl JasmDiagnostic {
    pub fn new(
        message: impl Into<String>,
        range: Option<Range<usize>>,
        note: Option<String>,
        label: Option<String>,
    ) -> Self {
        Self {
            message: message.into(),
            range,
            note,
            label,
        }
    }

    pub fn message(&self) -> &str {
        &self.message
    }

    pub fn range(&self) -> Option<&Range<usize>> {
        self.range.as_ref()
    }

    pub fn note(&self) -> Option<&str> {
        self.note.as_deref()
    }

    pub fn label(&self) -> Option<&str> {
        self.label.as_deref()
    }
}

impl From<LexerError> for JasmError {
    fn from(err: LexerError) -> Self {
        JasmError::Diagnostic(JasmDiagnostic::new(
            err.message().unwrap_or("lexing error".to_string()),
            err.as_range(),
            err.note(),
            err.label(),
        ))
    }
}

impl From<ParserError> for JasmError {
    fn from(err: ParserError) -> Self {
        match err {
            ParserError::Internal(msg) => JasmError::Internal(msg),
            _ => JasmError::Diagnostic(JasmDiagnostic::new(
                err.message().unwrap_or("parsing error".to_string()),
                err.as_range(),
                err.note(),
                err.label(),
            )),
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

    #[test]
    fn test_parser_internal_error_converts_to_jasm_internal() {
        let parser_err = ParserError::Internal("parser broke".to_string());
        let jasm_err = JasmError::from(parser_err);

        assert!(matches!(jasm_err, JasmError::Internal(msg) if msg == "parser broke"));
    }
}
