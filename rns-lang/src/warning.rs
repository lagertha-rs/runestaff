use ariadne::{Color, Label, Report, ReportKind, Source};
use std::ops::Range;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JasmWarning {
    message: String,
    primary_location: Range<usize>,
    label: Vec<(Range<usize>, String)>,
    note: String,
}

impl JasmWarning {
    pub fn new(
        message: impl Into<String>,
        primary_location: Range<usize>,
        label: Vec<(Range<usize>, String)>,
        note: String,
    ) -> Self {
        Self {
            message: message.into(),
            primary_location,
            note,
            label,
        }
    }

    pub fn message(&self) -> &str {
        &self.message
    }

    pub fn primary_location(&self) -> &Range<usize> {
        &self.primary_location
    }

    pub fn note(&self) -> &str {
        self.note.as_str()
    }

    pub fn labels(&self) -> &Vec<(Range<usize>, String)> {
        &self.label
    }
}
